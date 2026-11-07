use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::config::AppPaths;

/// Initializes structured logging to a daily rolling file.
///
/// Returns the appender guard; it must be kept alive for the process lifetime.
pub fn init(paths: &AppPaths) -> Option<WorkerGuard> {
    let file_appender = tracing_appender::rolling::daily(&paths.logs_dir, "wexo.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wexo=debug,wexo_app=debug"));

    let result = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .try_init();

    if result.is_err() {
        // A subscriber is already installed (e.g. during tests); keep the guard.
        eprintln!("tracing subscriber already initialized");
    }

    Some(guard)
}
