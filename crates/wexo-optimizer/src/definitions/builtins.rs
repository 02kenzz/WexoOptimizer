//! Built-in optimization definitions.

use std::fs;
use std::path::{Path, PathBuf};
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
        available: false,
        requires_admin: true, // Set at runtime
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
        requires_admin: true,
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
        requires_admin: true,
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
        requires_admin: true,
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
        requires_admin: true,
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
            registry_keys: vec!["HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects".to_string(),
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced".to_string()],
            services: vec![],
            files: vec![],
        },
        available: false,
        requires_admin: false,
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
            // The snapshot restores whatever existed before; this only removes the
            // values the tweak itself may have created.
            let advanced = RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
                .open_subkey_with_flags(
                    r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
                    KEY_ALL_ACCESS,
                )
                .map_err(|error| error.to_string())?;
            for name in [
                "TaskbarAnimations",
                "ListviewAlphaSelect",
                "ListviewWatermark",
                "ListviewShadow",
                "DisableThumbnailPreview",
            ] {
                let _ = advanced.delete_value(name);
            }
            Ok(())
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
        requires_admin: true,
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

/// Creates the "clean-shader-cache" optimization.
pub fn clean_shader_cache() -> OptimizationDef {
    let meta = Optimization {
        id: "clean-shader-cache".to_string(),
        category: Category::FileSystem,
        title: "Limpieza de caché de sombreadores".to_string(),
        description: "Elimina los archivos compilados antiguos de DirectX y de los controladores gráficos (NVIDIA, AMD e Intel).".to_string(),
        rationale: "Las cachés de sombreadores se regeneran solas. Borrarlas libera espacio y elimina entradas corruptas que provocan tirones; el primer inicio de un juego puede tardar unos segundos más mientras recompila.".to_string(),
        risk: RiskLevel::VeryLow,
        requires_reboot: false,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec![],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec![],
            services: vec![],
            files: vec![],
        },
        available: false,
        requires_admin: false,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            let mut freed = 0u64;
            let mut removed = 0usize;
            let mut found = 0usize;

            for dir in shader_cache_dirs() {
                if !dir.is_dir() {
                    continue;
                }
                found += 1;
                let (bytes, files) = clear_directory(&dir);
                freed += bytes;
                removed += files;
            }

            if found == 0 {
                return Ok("No se encontraron cachés de sombreadores en este equipo".to_string());
            }

            Ok(format!(
                "Caché de sombreadores limpiada: {} liberados ({} archivos)",
                format_size(freed),
                removed
            ))
        })
        // Deletion is best effort: files in use by a running game stay and are
        // simply skipped, so the tweak counts as applied once it has run.
        .verify(|| Ok(VerificationResult::Verified))
        .rollback(|| Ok(()))
        .build()
}

/// Creates the "disable-cloud-clipboard" optimization.
pub fn disable_cloud_clipboard() -> OptimizationDef {
    let meta = Optimization {
        id: "disable-cloud-clipboard".to_string(),
        category: Category::Privacy,
        title: "Desactivar Portapapeles en la nube y su sincronización".to_string(),
        description: "Impide que Windows envíe a los servidores de Microsoft lo que copias con CTRL+C y desactiva la sincronización entre dispositivos.".to_string(),
        rationale: "Solo afecta a quien copia y pega entre varios equipos con la misma cuenta de Microsoft. El portapapeles local sigue funcionando con normalidad.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: false,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec![],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec![
                "HKCU\\Software\\Microsoft\\Clipboard".to_string(),
                "HKLM\\SOFTWARE\\Policies\\Microsoft\\Windows\\System".to_string(),
            ],
            services: vec![],
            files: vec![],
        },
        available: false,
        requires_admin: true,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            set_registry_value(
                winreg::enums::HKEY_CURRENT_USER,
                "Software\\Microsoft\\Clipboard",
                "EnableCloudClipboard",
                winreg::enums::RegType::REG_DWORD as u32,
                &0u32.to_le_bytes(),
            )?;
            set_registry_value(
                winreg::enums::HKEY_CURRENT_USER,
                "Software\\Microsoft\\Clipboard",
                "CloudClipboardAutomaticUpload",
                winreg::enums::RegType::REG_DWORD as u32,
                &0u32.to_le_bytes(),
            )?;
            // Machine-wide policy that also blocks the cross-device history.
            set_registry_value(
                winreg::enums::HKEY_LOCAL_MACHINE,
                "SOFTWARE\\Policies\\Microsoft\\Windows\\System",
                "AllowCrossDeviceClipboard",
                winreg::enums::RegType::REG_DWORD as u32,
                &0u32.to_le_bytes(),
            )?;
            Ok("Portapapeles en la nube y sincronización entre dispositivos desactivados".to_string())
        })
        .verify(|| {
            let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
            let key = hkcu
                .open_subkey_with_flags("Software\\Microsoft\\Clipboard", KEY_READ)
                .map_err(|e| e.to_string())?;
            let cloud: u32 = key.get_value("EnableCloudClipboard").unwrap_or(1);
            let upload: u32 = key.get_value("CloudClipboardAutomaticUpload").unwrap_or(1);
            if cloud == 0 && upload == 0 {
                Ok(VerificationResult::Verified)
            } else {
                Ok(VerificationResult::NotApplied)
            }
        })
        .rollback(|| {
            // The snapshot restores any value that already existed; this only
            // removes the ones the tweak may have created.
            let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
            if let Ok(key) =
                hkcu.open_subkey_with_flags("Software\\Microsoft\\Clipboard", KEY_ALL_ACCESS)
            {
                let _ = key.delete_value("EnableCloudClipboard");
                let _ = key.delete_value("CloudClipboardAutomaticUpload");
            }
            let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
            if let Ok(key) = hklm.open_subkey_with_flags(
                "SOFTWARE\\Policies\\Microsoft\\Windows\\System",
                KEY_ALL_ACCESS,
            ) {
                let _ = key.delete_value("AllowCrossDeviceClipboard");
            }
            Ok(())
        })
        .build()
}

/// Creates the "flush-dns-reset-winsock" optimization.
pub fn flush_dns_reset_winsock() -> OptimizationDef {
    let meta = Optimization {
        id: "flush-dns-reset-winsock".to_string(),
        category: Category::Network,
        title: "Vaciar caché DNS y restablecer Winsock".to_string(),
        description: "Limpia la tabla de resolución de nombres de dominio y reinicia la pila de red de Windows.".to_string(),
        rationale: "Resuelve fallos de carga de webs y conexiones sin perder la configuración del adaptador. El restablecimiento de Winsock se completa al reiniciar.".to_string(),
        risk: RiskLevel::Low,
        requires_reboot: true,
        applies_to: Applicability {
            os_min_build: Some("19044".to_string()),
            hardware_filters: vec![],
            conflicts_with: vec![],
        },
        snapshot: SnapshotSpec {
            registry_keys: vec![],
            services: vec![],
            files: vec![],
        },
        available: false,
        requires_admin: true,
    };

    OptimizationDef::builder(meta)
        .apply(|| {
            let flush = Command::new("ipconfig.exe")
                .arg("/flushdns")
                .output()
                .map_err(|e| e.to_string())?;
            if !flush.status.success() {
                return Err(String::from_utf8_lossy(&flush.stderr).to_string());
            }

            let reset = Command::new("netsh.exe")
                .args(["winsock", "reset"])
                .output()
                .map_err(|e| e.to_string())?;
            if !reset.status.success() {
                return Err(String::from_utf8_lossy(&reset.stderr).to_string());
            }

            Ok("Caché DNS vaciada y Winsock restablecido. Reinicia para completarlo".to_string())
        })
        // Both commands report their own success; there is no persistent state
        // to inspect afterwards.
        .verify(|| Ok(VerificationResult::Verified))
        .rollback(|| Ok(()))
        .build()
}

/// Directories that hold compiled shader caches for DirectX and the main GPU
/// vendors. Missing directories are simply skipped.
fn shader_cache_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        dirs.push(local.join("D3DSCache"));
        dirs.push(local.join("NVIDIA").join("DXCache"));
        dirs.push(local.join("NVIDIA").join("GLCache"));
        dirs.push(local.join("AMD").join("DxCache"));
        dirs.push(local.join("AMD").join("DxcCache"));
        dirs.push(local.join("AMD").join("GLCache"));
        dirs.push(local.join("AMD").join("VkCache"));
        dirs.push(local.join("Intel").join("ShaderCache"));
    }
    if let Some(program_data) = std::env::var_os("ProgramData").map(PathBuf::from) {
        dirs.push(program_data.join("NVIDIA Corporation").join("NV_Cache"));
    }
    dirs
}

/// Removes the contents of `dir` and returns the bytes and files removed.
/// Files locked by a running process are skipped instead of failing the tweak.
fn clear_directory(dir: &Path) -> (u64, usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return (0, 0);
    };

    let mut freed = 0u64;
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        let size = if is_dir {
            directory_size(&path)
        } else {
            entry.metadata().map(|meta| meta.len()).unwrap_or(0)
        };

        let deleted = if is_dir {
            fs::remove_dir_all(&path).is_ok()
        } else {
            fs::remove_file(&path).is_ok()
        };
        if deleted {
            freed += size;
            removed += 1;
        }
    }
    (freed, removed)
}

/// Total size of the files below `dir`, without following errors.
fn directory_size(dir: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            let path = entry.path();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                directory_size(&path)
            } else {
                entry.metadata().map(|meta| meta.len()).unwrap_or(0)
            }
        })
        .sum()
}

/// Human-readable size for the apply message.
fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let value = bytes as f64;
    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.0} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
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