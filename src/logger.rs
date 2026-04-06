//! Logging initialization using `tracing` crate.
use std::path::Path;
use std::sync::{Once, OnceLock};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::Layer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the logger with daily rolling file appender and console output.
pub fn init_logger(log_dir: impl AsRef<Path>) {
    static INIT: Once = Once::new();
    static GUARD: OnceLock<WorkerGuard> = OnceLock::new();
    INIT.call_once(|| {
        let file_appender = rolling::daily(log_dir, "judger.log");

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = fmt::layer()
            .with_ansi(false)
            .with_writer(non_blocking)
            .with_filter(EnvFilter::new("trace"));

        let stdout_layer = fmt::layer()
            .pretty()
            .with_writer(std::io::stdout)
            .with_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
            );

        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .init();

        #[allow(clippy::expect_used)]
        GUARD.set(guard).expect("Logger guard already set");
    })
}
