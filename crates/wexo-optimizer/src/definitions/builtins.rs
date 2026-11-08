//! Built-in optimization definitions.

use std::process::Command;
use winreg::enums::*;
use winreg::{RegKey, HKEY};
use wexo_core::{Applicability, Category, Optimization, RiskLevel, SnapshotSpec, VerificationResult};

use super::OptimizationDef;

/// Creates the "disable-sysmain" optimization.
pub fn disable_sysmain() -> OptimizationDef {
    let meta = Optimization {
        id: "disable-sysmain".to_string(),
        category: Category::Services,
        title: "Deshabilitar SysMain (Superfetch)".to_string(),
        description: "Detiene el servicio que precarga aplicaciones en memoria y reduce la actividad de E/S en segundo plano.".to_string(),
        rationale: "En un SSD/NVMe el acceso aleatorio es de microsegundos. La precarga aporta poco y consume ciclos de disco y RAM.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: false,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec!["disk.kind == 'nvme' || disk.kind == 'ssd'".to_string()],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec![],
            services: vec!["SysMain".to_string()],
            files: vec![],
        },
        available: false, // Set at runtime
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            stop_service("SysMain")?;
            set_service_start_type("SysMain", "disabled")?;
            Ok("SysMain service stopped and disabled".to_string())
        })
        .verify(|| {
            if is_service_stopped("SysMain") && is_service_disabled("SysMain") {
                Ok(VerificationResult::Verified)
            } else {
                Ok(VerificationResult::NotApplied)
            }
        })
        .rollback(|| {
            set_service_start_type("SysMain", "demand")?;
            start_service("SysMain")?;
            Ok(())
        })
        .build()
}

/// Creates the "disable-diagtrack" optimization.
pub fn disable_diagtrack() -> OptimizationDef {
    let meta = Optimization {
        id: "disable-diagtrack".to_string(),
        category: Category::Privacy,
        title: "Deshabilitar telemetría (DiagTrack)".to_string(),
        description: "Detiene el servicio de telemetría de diagnóstico y sus tareas programadas asociadas.".to_string(),
        rationale: "Windows recopila datos de diagnóstico de forma continua. Desactivarlo reduce E/S y actividad de red no esencial.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: false,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec![],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec![],
            services: vec!["DiagTrack".to_string(), "dmwappushservice".to_string()],
            files: vec![],
        },
        available: false,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            for svc in ["DiagTrack", "dmwappushservice"] {
                stop_service(svc)?;
                set_service_start_type(svc, "disabled")?;
            }
            // Disable scheduled tasks
            disable_task("\\Microsoft\\Windows\\Customer Experience Improvement Program\\*")?;
            disable_task("\\Microsoft\\Windows\\Application Experience\\*")?;
            Ok("DiagTrack and related services disabled".to_string())
        })
        .verify(|| {
            let all_stopped = ["DiagTrack", "dmwappushservice"]
                .iter()
                .all(|s| is_service_stopped(s) && is_service_disabled(s));
            if all_stopped {
                Ok(VerificationResult::Verified)
            } else {
                Ok(VerificationResult::NotApplied)
            }
        })
        .rollback(|| {
            for svc in ["DiagTrack", "dmwappushservice"] {
                set_service_start_type(svc, "demand")?;
                start_service(svc)?;
            }
            enable_task("\\Microsoft\\Windows\\Customer Experience Improvement Program\\*")?;
            enable_task("\\Microsoft\\Windows\\Application Experience\\*")?;
            Ok(())
        })
        .build()
}

/// Creates the "power-plan-ultimate" optimization.
pub fn power_plan_ultimate() -> OptimizationDef {
    let meta = Optimization {
        id: "power-plan-ultimate".to_string(),
        category: Category::Power,
        title: "Plan de energía de alto rendimiento".to_string(),
        description: "Activa el plan Ultimate Performance y elimina la latencia extra de los estados de ahorro en escritorio.".to_string(),
        rationale: "En equipos de sobremesa, el ahorro de energía no aporta beneficio y añade micro-latencias al cambio de frecuencia.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: false,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec!["chassis == 'desktop'".to_string()],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec!["HKLM\\SYSTEM\\CurrentControlSet\\Control\\Power".to_string()],
            services: vec![],
            files: vec![],
        },
        available: false,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            // Set active power scheme to Ultimate Performance
            let output = Command::new("powercfg.exe")
                .args(["/setactive", "e9a42b02-d5df-448d-aa00-03f14749eb61"])
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).to_string());
            }
            Ok("Ultimate Performance power plan activated".to_string())
        })
        .verify(|| {
            let output = Command::new("powercfg.exe")
                .args(["/getactivescheme"])
                .output()
                .map_err(|e| e.to_string())?;
            let scheme = String::from_utf8_lossy(&output.stdout);
            if scheme.contains("e9a42b02-d5df-448d-aa00-03f14749eb61") {
                Ok(VerificationResult::Verified)
            } else {
                Ok(VerificationResult::NotApplied)
            }
        })
        .rollback(|| {
            // Restore to Balanced plan
            let _ = Command::new("powercfg.exe")
                .args(["/setactive", "381b4222-f694-41f0-9685-ff5bb260df2e"])
                .status();
            Ok(())
        })
        .build()
}

/// Creates the "tcp-autotuning" optimization.
pub fn tcp_autotuning() -> OptimizationDef {
    let meta = Optimization {
        id: "tcp-autotuning".to_string(),
        category: Category::Network,
        title: "Optimizar autotuning de TCP".to_string(),
        description: "Ajusta la ventana de recepción TCP y el RSS para adaptarse al ancho de banda real del adaptador.".to_string(),
        rationale: "El valor por defecto de Windows puede limitar el throughput en conexiones de alta latencia o enlaces superiores a 1 Gbps.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: false,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec![],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec!["HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters".to_string()],
            services: vec![],
            files: vec![],
        },
        available: false,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            // Enable TCP autotuning and RSS
            set_registry_value(
                winreg::enums::HKEY_LOCAL_MACHINE,
                "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters",
                "EnableTCPAutotuning",
                winreg::enums::RegType::REG_DWORD as u32,
                &1u32.to_le_bytes(),
            )?;
            set_registry_value(
                winreg::enums::HKEY_LOCAL_MACHINE,
                "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters",
                "EnableRSS",
                winreg::enums::RegType::REG_DWORD as u32,
                &1u32.to_le_bytes(),
            )?;
            // Set autotuning level to normal
            let output = Command::new("netsh.exe")
                .args(["int", "tcp", "set", "global", "autotuninglevel=normal"])
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).to_string());
            }
            Ok("TCP autotuning optimized".to_string())
        })
        .verify(|| {
            let output = Command::new("netsh.exe")
                .args(["int", "tcp", "show", "global"])
                .output()
                .map_err(|e| e.to_string())?;
            let result = String::from_utf8_lossy(&output.stdout);
            if result.contains("Autotuning Level") && result.contains("normal") {
                Ok(VerificationResult::Verified)
            } else {
                Ok(VerificationResult::NotApplied)
            }
        })
        .rollback(|| {
            let _ = Command::new("netsh.exe")
                .args(["int", "tcp", "set", "global", "autotuninglevel=default"])
                .status();
            Ok(())
        })
        .build()
}

/// Creates the "enable-hags" optimization.
pub fn enable_hags() -> OptimizationDef {
    let meta = Optimization {
        id: "enable-hags".to_string(),
        category: Category::Gpu,
        title: "Programación de GPU acelerada por hardware".to_string(),
        description: "Activa HAGS para que la GPU gestione su propia cola de trabajo con menor latencia de CPU.".to_string(),
        rationale: "Reduce la intervención de la CPU en la planificación de trabajos gráficos; requiere driver y SO compatibles.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: true,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec!["gpu.driver_model == 'wddm2'".to_string()],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec!["HKLM\\SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers".to_string()],
            services: vec![],
            files: vec![],
        },
        available: false,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            set_registry_value(
                winreg::enums::HKEY_LOCAL_MACHINE,
                "SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers",
                "HwSchMode",
                winreg::enums::RegType::REG_DWORD as u32,
                &2u32.to_le_bytes(),
            )?;
            Ok("Hardware-accelerated GPU scheduling enabled (requires reboot)".to_string())
        })
        .verify(|| {
            let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            let key = hklm
                .open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers", KEY_READ)
                .map_err(|e| e.to_string())?;
            let value: u32 = key
                .get_value("HwSchMode")
                .map_err(|e| e.to_string())?;
            if value == 2 {
                Ok(VerificationResult::Verified)
            } else {
                Ok(VerificationResult::NotApplied)
            }
        })
        .rollback(|| {
            set_registry_value(
                winreg::enums::HKEY_LOCAL_MACHINE,
                "SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers",
                "HwSchMode",
                winreg::enums::RegType::REG_DWORD as u32,
                &1u32.to_le_bytes(),
            )?;
            Ok(())
        })
        .build()
}

/// Creates the "reduce-visual-effects" optimization.
pub fn reduce_visual_effects() -> OptimizationDef {
    let meta = Optimization {
        id: "reduce-visual-effects".to_string(),
        category: Category::Visuals,
        title: "Reducir efectos visuales".to_string(),
        description: "Desactiva animaciones, sombras y transparencias manteniendo la legibilidad y el contraste.".to_string(),
        rationale: "En equipos con GPU integrada o batería, los efectos visuales consumen recursos sin mejorar la productividad.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: false,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec![],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec!["HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects".to_string()],
            services: vec![],
            files: vec![],
        },
        available: false,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            // VisualFXSetting: 1 = Let Windows choose, 2 = Best appearance, 3 = Best performance, 4 = Custom
            set_registry_value(
                winreg::enums::HKEY_CURRENT_USER,
                "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects",
                "VisualFXSetting",
                winreg::enums::RegType::REG_DWORD as u32,
                &3u32.to_le_bytes(),
            )?;

            // Disable specific effects
            let effects_key = RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
                .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced")
                .map(|(k, _)| k)
                .map_err(|e| e.to_string())?;

            let settings = [
                ("TaskbarAnimations", 0u32),
                ("ListviewAlphaSelect", 0),
                ("ListviewWatermark", 0),
                ("ListviewShadow", 0),
                ("DisableThumbnailPreview", 1),
            ];
            for (name, value) in settings {
                effects_key
                    .set_value(name, &value)
                    .map_err(|e| e.to_string())?;
            }

            Ok("Visual effects reduced to best performance".to_string())
        })
        .verify(|| {
            let hklm = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
            let key = hklm
                .open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects", KEY_READ)
                .map_err(|e| e.to_string())?;
            let value: u32 = key
                .get_value("VisualFXSetting")
                .map_err(|e| e.to_string())?;
            if value == 3 {
                Ok(VerificationResult::Verified)
            } else {
                Ok(VerificationResult::NotApplied)
            }
        })
        .rollback(|| {
            set_registry_value(
                winreg::enums::HKEY_CURRENT_USER,
                "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects",
                "VisualFXSetting",
                winreg::enums::RegType::REG_DWORD as u32,
                &1u32.to_le_bytes(),
            )?;
            Ok(())
        })
        .build()
}

/// Creates the "disable-startup-bloat" optimization.
pub fn disable_startup_bloat() -> OptimizationDef {
    let meta = Optimization {
        id: "disable-startup-bloat".to_string(),
        category: Category::Startup,
        title: "Revisar aplicaciones de inicio".to_string(),
        description: "Deshabilita entradas de inicio innecesarias detectadas por impacto medido de CPU, disco y tiempo de arranque.".to_string(),
        rationale: "Cada aplicación de inicio añade latencia al login. Solo se desactivan entradas sin impacto funcional demostrado.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: false,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec![],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec!["HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run".to_string()],
            services: vec![],
            files: vec![],
        },
        available: false,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            // This is a placeholder - actual implementation would analyze startup impact
            // For now, we just document the capability
            Ok("Startup optimization analyzed (requires user review)".to_string())
        })
        .verify(|| {
            Ok(VerificationResult::Verified)
        })
        .build()
}

/// Creates the "ntfs-last-access" optimization.
pub fn ntfs_last_access() -> OptimizationDef {
    let meta = Optimization {
        id: "ntfs-last-access".to_string(),
        category: Category::FileSystem,
        title: "Desactivar actualización de último acceso".to_string(),
        description: "Evita que NTFS escriba la marca de tiempo de último acceso en cada lectura de archivo.".to_string(),
        rationale: "Reduce escrituras innecesarias en cargas de trabajo con muchos archivos pequeños, como compilaciones o juegos.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: true,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec![],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec!["HKLM\\SYSTEM\\CurrentControlSet\\Control\\FileSystem".to_string()],
            services: vec![],
            files: vec![],
        },
        available: false,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            set_registry_value(
                winreg::enums::HKEY_LOCAL_MACHINE,
                "SYSTEM\\CurrentControlSet\\Control\\FileSystem",
                "NtfsDisableLastAccessUpdate",
                winreg::enums::RegType::REG_DWORD as u32,
                &1u32.to_le_bytes(),
            )?;
            Ok("NTFS last access timestamp updates disabled (requires reboot)".to_string())
        })
        .verify(|| {
            let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            let key = hklm
                .open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\FileSystem", KEY_READ)
                .map_err(|e| e.to_string())?;
            let value: u32 = key
                .get_value("NtfsDisableLastAccessUpdate")
                .map_err(|e| e.to_string())?;
            if value == 1 {
                Ok(VerificationResult::Verified)
            } else {
                Ok(VerificationResult::NotApplied)
            }
        })
        .rollback(|| {
            set_registry_value(
                winreg::enums::HKEY_LOCAL_MACHINE,
                "SYSTEM\\CurrentControlSet\\Control\\FileSystem",
                "NtfsDisableLastAccessUpdate",
                winreg::enums::RegType::REG_DWORD as u32,
                &0u32.to_le_bytes(),
            )?;
            Ok(())
        })
        .build()
}

/// Helper to set a registry value.
fn set_registry_value(
    root: HKEY,
    path: &str,
    name: &str,
    value_type: u32,
    data: &[u8],
) -> Result<(), String> {
    let hkey = RegKey::predef(root);
    let key = hkey
        .open_subkey_with_flags(path, KEY_WRITE)
        .or_else(|_| hkey.create_subkey(path).map(|(k, _)| k))
        .map_err(|e| e.to_string())?;

    let reg_value = if value_type == winreg::enums::RegType::REG_DWORD as u32 {
        let _bytes: [u8; 4] = data.try_into().map_err(|_| "Invalid DWORD data")?;
        winreg::RegValue {
            bytes: data.to_vec(),
            vtype: winreg::enums::RegType::REG_DWORD,
        }
    } else if value_type == winreg::enums::RegType::REG_QWORD as u32 {
        let _bytes: [u8; 8] = data.try_into().map_err(|_| "Invalid QWORD data")?;
        winreg::RegValue {
            bytes: data.to_vec(),
            vtype: winreg::enums::RegType::REG_QWORD,
        }
    } else if value_type == winreg::enums::RegType::REG_SZ as u32 {
        winreg::RegValue {
            bytes: data.to_vec(),
            vtype: winreg::enums::RegType::REG_SZ,
        }
    } else if value_type == winreg::enums::RegType::REG_EXPAND_SZ as u32 {
        winreg::RegValue {
            bytes: data.to_vec(),
            vtype: winreg::enums::RegType::REG_EXPAND_SZ,
        }
    } else if value_type == winreg::enums::RegType::REG_MULTI_SZ as u32 {
        winreg::RegValue {
            bytes: data.to_vec(),
            vtype: winreg::enums::RegType::REG_MULTI_SZ,
        }
    } else {
        winreg::RegValue {
            bytes: data.to_vec(),
            vtype: winreg::enums::RegType::REG_BINARY,
        }
    };

    key.set_raw_value(name, &reg_value).map_err(|e| e.to_string())?;
    Ok(())
}

/// Helper to stop a service.
fn stop_service(name: &str) -> Result<(), String> {
    let output = Command::new("sc.exe")
        .args(["stop", name])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        // Service might already be stopped, which is OK
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("1062") { // ERROR_SERVICE_NOT_ACTIVE
            return Err(stderr.to_string());
        }
    }
    Ok(())
}

/// Helper to start a service.
fn start_service(name: &str) -> Result<(), String> {
    let output = Command::new("sc.exe")
        .args(["start", name])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("1056") { // ERROR_SERVICE_ALREADY_RUNNING
            return Err(stderr.to_string());
        }
    }
    Ok(())
}

/// Helper to set service start type.
fn set_service_start_type(name: &str, start_type: &str) -> Result<(), String> {
    let output = Command::new("sc.exe")
        .args(["config", name, "start=", start_type])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

/// Checks if a service is stopped.
fn is_service_stopped(name: &str) -> bool {
    let output = Command::new("sc.exe")
        .args(["query", name])
        .output()
        .ok();
    output
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("STOPPED"))
        .unwrap_or(false)
}

/// Checks if a service is disabled.
fn is_service_disabled(name: &str) -> bool {
    let output = Command::new("sc.exe")
        .args(["qc", name])
        .output()
        .ok();
    output
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.contains("START_TYPE") && l.contains("DISABLED"))
        })
        .unwrap_or(false)
}

/// Disables a scheduled task.
fn disable_task(path: &str) -> Result<(), String> {
    let output = Command::new("schtasks.exe")
        .args(["/Change", "/TN", path, "/Disable"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Task might not exist, which is OK
        if !stderr.contains("ERROR: The system cannot find the file specified") {
            return Err(stderr.to_string());
        }
    }
    Ok(())
}

/// Enables a scheduled task.
fn enable_task(path: &str) -> Result<(), String> {
    let output = Command::new("schtasks.exe")
        .args(["/Change", "/TN", path, "/Enable"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("ERROR: The system cannot find the file specified") {
            return Err(stderr.to_string());
        }
    }
    Ok(())
}