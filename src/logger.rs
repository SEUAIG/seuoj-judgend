//! Logging initialization using `tracing` crate.
use std::path::Path;
use std::sync::{Once, OnceLock};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the logger with daily rolling file appender and console output.
pub fn init_logger(log_dir: impl AsRef<Path>) {
    static INIT: Once = Once::new();
    static GUARD: OnceLock<WorkerGuard> = OnceLock::new();
    INIT.call_once(|| {
        let file_appender = rolling::daily(log_dir, "judger.log");

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_writer(std::io::stdout).pretty())
            .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
            .init();

        if GUARD.set(guard).is_err() {
            eprintln!("Warning: Logger guard already initialized.");
        }
    })
}
