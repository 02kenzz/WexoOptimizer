//! Linux optimizations: GNOME visuals, systemd services and kernel tuning.
//!
//! Every definition drives a tool that ships with mainstream distributions
//! (`gsettings`, `systemctl`, `sysctl`) and reads the real result back, so a
//! machine without GNOME or systemd marks the item as unavailable instead of
//! pretending to change something.

use std::process::Command;

use wexo_core::{
    Applicability, Category, Optimization, RiskLevel, SnapshotSpec, VerificationResult,
};

use super::OptimizationDef;

const GNOME_SCHEMA: &str = "org.gnome.desktop.interface";
const GNOME_ANIMATIONS_KEY: &str = "enable-animations";
const SWAPPINESS_FILE: &str = "/etc/sysctl.d/99-wexo-swappiness.conf";
const SWAPPINESS_TARGET: u32 = 10;

/// Runs a command and fails when it exits non-zero.
fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("no se pudo ejecutar {program}: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("{program} {}: {stderr}", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Runs a command and returns its output plus whether it succeeded.
///
/// `systemctl is-active` exits non-zero for inactive units, which is a valid
/// answer rather than an error.
fn output(program: &str, args: &[&str]) -> Result<(String, bool), String> {
    let result = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("no se pudo ejecutar {program}: {error}"))?;
    let text = String::from_utf8_lossy(&result.stdout).trim().to_string();
    Ok((text, result.status.success()))
}

#[allow(clippy::too_many_arguments)]
fn base(
    id: &str,
    category: Category,
    title: &str,
    description: &str,
    rationale: &str,
    requires_reboot: bool,
    requires_admin: bool,
    hardware_filters: &[&str],
    snapshot: SnapshotSpec,
) -> Optimization {
    Optimization {
        id: id.to_string(),
        category,
        title: title.to_string(),
        description: description.to_string(),
        rationale: rationale.to_string(),
        risk: RiskLevel::Low,
        requires_reboot,
        applies_to: Applicability {
            os_min_build: None,
            hardware_filters: hardware_filters
                .iter()
                .map(|filter| filter.to_string())
                .collect(),
            conflicts_with: Vec::new(),
        },
        snapshot,
        available: false,
        requires_admin,
    }
}

/// Disables GNOME shell animations: user-level and immediately reversible.
pub fn reduce_visual_effects() -> OptimizationDef {
    let meta = base(
        "reduce-visual-effects",
        Category::Visuals,
        "Reducir animaciones del escritorio",
        "Desactiva las animaciones de GNOME para que las ventanas y los espacios de trabajo cambien al instante.",
        "En equipos con gráficos integrados cada animación consume GPU y CPU; desactivarlas hace la interfaz más ágil sin perder legibilidad.",
        false,
        false,
        &["platform.linux", "desktop.gnome"],
        SnapshotSpec::default(),
    );

    OptimizationDef::builder(meta)
        .apply(|| {
            run("gsettings", &["set", GNOME_SCHEMA, GNOME_ANIMATIONS_KEY, "false"])?;
            Ok("Animaciones de GNOME desactivadas".to_string())
        })
        .verify(|| {
            let value = run("gsettings", &["get", GNOME_SCHEMA, GNOME_ANIMATIONS_KEY])?;
            if value.trim() == "false" {
                Ok(VerificationResult::Verified)
            } else {
                Ok(VerificationResult::NotApplied)
            }
        })
        .rollback(|| {
            run("gsettings", &["set", GNOME_SCHEMA, GNOME_ANIMATIONS_KEY, "true"]).map(|_| ())
        })
        .build()
}

/// Stops and disables the Bluetooth stack through systemd.
pub fn disable_bluetooth() -> OptimizationDef {
    let meta = base(
        "disable-bluetooth",
        Category::Services,
        "Desactivar Bluetooth",
        "Detiene y deshabilita el servicio Bluetooth para liberar memoria y evitar que la radio consuma batería.",
        "Si no se usan periféricos Bluetooth, el servicio mantiene la radio y un proceso residente sin aportar nada.",
        false,
        true,
        &["platform.linux", "systemd", "service.bluetooth"],
        SnapshotSpec {
            registry_keys: Vec::new(),
            services: vec!["bluetooth".to_string()],
            files: Vec::new(),
        },
    );

    OptimizationDef::builder(meta)
        .apply(|| {
            run("systemctl", &["disable", "--now", "bluetooth"])?;
            Ok("Servicio Bluetooth desactivado".to_string())
        })
        .verify(|| {
            let (state, active) = output("systemctl", &["is-active", "bluetooth"])?;
            if active && state == "active" {
                Ok(VerificationResult::NotApplied)
            } else {
                Ok(VerificationResult::Verified)
            }
        })
        .rollback(|| run("systemctl", &["enable", "--now", "bluetooth"]).map(|_| ()))
        .build()
}

/// Lowers `vm.swappiness` through a drop-in sysctl file.
pub fn vm_swappiness() -> OptimizationDef {
    let meta = base(
        "vm-swappiness",
        Category::Memory,
        "Reducir el uso de swap",
        "Baja vm.swappiness a 10 para que el sistema prefiera mantener en RAM las páginas activas.",
        "Con poca memoria, el valor por defecto expulsa a swap páginas que se van a volver a usar y provoca tirones.",
        false,
        true,
        &["platform.linux", "writable.sysctl"],
        SnapshotSpec {
            registry_keys: Vec::new(),
            services: Vec::new(),
            files: vec![SWAPPINESS_FILE.to_string()],
        },
    );

    OptimizationDef::builder(meta)
        .apply(|| {
            let contents = format!("# Wexo Optimizer\nvm.swappiness={SWAPPINESS_TARGET}\n");
            std::fs::write(SWAPPINESS_FILE, contents)
                .map_err(|error| format!("no se pudo escribir {SWAPPINESS_FILE}: {error}"))?;
            run("sysctl", &["--system"])?;
            Ok(format!("vm.swappiness = {SWAPPINESS_TARGET}"))
        })
        .verify(|| {
            let raw = std::fs::read_to_string("/proc/sys/vm/swappiness")
                .map_err(|error| format!("no se pudo leer /proc/sys/vm/swappiness: {error}"))?;
            match raw.trim().parse::<u32>() {
                Ok(value) if value == SWAPPINESS_TARGET => Ok(VerificationResult::Verified),
                Ok(value) => Ok(VerificationResult::Mismatch {
                    details: format!("vm.swappiness = {value}"),
                }),
                Err(error) => Err(format!("valor de swappiness inválido: {error}")),
            }
        })
        .rollback(|| {
            let _ = std::fs::remove_file(SWAPPINESS_FILE);
            run("sysctl", &["--system"]).map(|_| ())
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_are_well_formed() {
        for def in [reduce_visual_effects(), disable_bluetooth(), vm_swappiness()] {
            assert!(!def.meta.id.is_empty());
            assert!(!def.meta.title.is_empty());
            assert!(!def.meta.available, "availability is filled by the registry");
        }
    }

    #[test]
    fn bluetooth_snapshot_captures_the_unit() {
        let def = disable_bluetooth();
        assert_eq!(def.meta.snapshot.services, vec!["bluetooth".to_string()]);
    }
}
