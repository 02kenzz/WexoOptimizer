//! Startup elevation: the app always opens with administrator rights.
//!
//! A running process cannot change its own token, so an unelevated instance
//! starts an elevated copy of itself and exits. [`NO_ELEVATE`] marks that copy
//! (and any manual relaunch) so a failed elevation can never loop.

use std::process::Command;

/// Argument passed to the elevated copy so it does not try to elevate again.
pub const NO_ELEVATE: &str = "--no-elevate";

/// True when this process can already run privileged optimizations.
pub fn already_elevated() -> bool {
    wexo_sysinfo::is_elevated()
}

/// True when the app must stop because an elevated copy is taking over.
///
/// Returns `false` when already elevated, when the `--no-elevate` marker is
/// present, or when elevation was declined/failed — in that last case the app
/// keeps running without the admin-only tweaks.
pub fn should_hand_off() -> bool {
    if already_elevated() || std::env::args().any(|arg| arg == NO_ELEVATE) {
        return false;
    }
    relaunch_elevated()
}

/// Starts an elevated copy of this executable.
///
/// Returns `true` when the copy was launched, meaning the caller has to exit.
pub fn relaunch_elevated() -> bool {
    #[cfg(windows)]
    {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        // `Start-Process` returns once the UAC prompt is answered; a declined
        // prompt exits non-zero, which is handled by the caller as "keep going".
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &elevation_script(&exe),
            ])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    {
        // polkit never re-prompts an already-root process.
        if std::env::var_os("PKEXEC_UID").is_some() {
            return false;
        }
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        Command::new("pkexec")
            .arg(&exe)
            .arg(NO_ELEVATE)
            .spawn()
            .is_ok()
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        false
    }
}

/// Builds the PowerShell command that hands over to the elevated copy.
///
/// Single quotes are doubled because they are the only quoting PowerShell
/// interprets inside a single-quoted string.
#[cfg(windows)]
fn elevation_script(exe: &std::path::Path) -> String {
    let path = exe.display().to_string().replace('\'', "''");
    format!("Start-Process -FilePath '{path}' -ArgumentList '{NO_ELEVATE}' -Verb RunAs")
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn startup_script_quotes_apostrophes_and_passes_the_marker() {
        let script = elevation_script(std::path::Path::new("C:/Program Files/O'Brien/wexo-app.exe"));
        assert!(script.contains("O''Brien"), "{script}");
        assert!(script.contains(NO_ELEVATE), "{script}");
        assert!(script.contains("-Verb RunAs"), "{script}");
        assert!(script.starts_with("Start-Process -FilePath '"), "{script}");
    }
}
