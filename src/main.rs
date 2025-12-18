#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
//! SEU AIJ Judge-Endpoint

use crate::config::init_config;
use crate::logger::init_logger;
use axum::Router;
use axum::routing::{get, post};
mod config;
mod error;
mod fs;
mod judger;
mod logger;
mod server;

use crate::fs::FileSystem;
use crate::server::{get_problem_by_id, judge_problem_by_id};
use error::Result;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let config = init_config().await?;
    let _guard = init_logger(&config.log_dir);
    FileSystem::init(&config.problems_dir)?;
    info!(
        "FileSystem initialized with base path: {}",
        &config.problems_dir.to_string_lossy()
    );
    let app = Router::new()
        .route("/judge/problem/{pid}", get(get_problem_by_id))
        .route("/judge/submission", post(judge_problem_by_id));
    let listen_addr = format!("{}:{}", config.listen_addr, config.listen_port);
    let listener = TcpListener::bind(&listen_addr).await.map_err(|e| {
        error::AijError::Server(format!("Failed to bind to {}: {}", listen_addr, e))
    })?;
    info!("Server listening on {}", listen_addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            #[allow(clippy::expect_used)]
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            info!("Shutdown signal received, shutting down...");
        })
        .await
        .map_err(|e| error::AijError::Server(format!("Server error: {}", e)))?;
    info!("Server has shut down gracefully.");
    Ok(())
}
