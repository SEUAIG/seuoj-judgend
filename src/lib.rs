#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
//! SEU AIJ Judge-Endpoint

use crate::config::AijConfig;
use crate::server::{
    edit_problem_by_id, get_problem_by_id, judge_problem_by_id, serve_data_metadata,
    serve_problem_file, upload_problem_data,
};
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};

pub mod config;
pub mod error;
pub mod fs;
pub mod judger;
pub mod logger;
pub mod server;

/// Create the Axum application with defined routes.
pub fn app() -> Router {
    Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/judge/problem/{pid}", get(get_problem_by_id))
        .route("/judge/submission", post(judge_problem_by_id))
        .route("/judge/problem/edit", patch(edit_problem_by_id))
        .route("/judge/problem/data", post(upload_problem_data))
        .route(
            "/judge/problem/file/{pid}/{filename}",
            get(serve_problem_file),
        )
        .route("/judge/problem/data/{pid}", get(serve_data_metadata))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100 MB
}

/// initialize the application (configuration, logger, etc.)
pub async fn initialize() -> &'static AijConfig {
    let config = AijConfig::get();
    logger::init_logger(&config.log_dir);
    config.update_binary_path().await;
    config
}
