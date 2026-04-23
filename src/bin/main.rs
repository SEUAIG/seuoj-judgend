use aij_judgend::{app, initialize};
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = initialize().await;
    info!(
        "FileSystem initialized with base path: {}",
        &config.problems_dir.to_string_lossy()
    );
    let app = app();
    let listen_addr = format!("{}:{}", config.listen_addr, config.listen_port);
    let listener = TcpListener::bind(&listen_addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", &listen_addr, e))?;
    info!("Server listening on {}", listen_addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let ctrl_c = signal::ctrl_c();
            #[cfg(unix)]
            let mut sigterm =
                signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("Failed to install SIGTERM handler");
            #[cfg(unix)]
            tokio::select! {
                _ = ctrl_c => {}
                _ = sigterm.recv() => {}
            }
            #[cfg(not(unix))]
            ctrl_c.await.expect("Failed to install Ctrl+C handler");
            info!("Shutdown signal received, shutting down...");
        })
        .await
        .map_err(|e| format!("Server error: {}", e))?;
    info!("Server has shut down gracefully.");
    Ok(())
}
