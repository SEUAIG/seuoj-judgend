#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
//! SEU AIJ Judge-Endpoint

use crate::config::AijConfig;
use crate::server::{
    delete_problem_by_id, delete_problem_file, edit_problem_by_id, get_problem_by_id,
    get_problem_config, get_problem_file, get_problem_tree, get_submission_file,
    get_submission_tree, judge_problem_by_id, put_problem_config, upload_problem_data,
};
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, patch, post, put};
use tokio::sync::OnceCell;
use tower_http::trace::TraceLayer;

pub mod config;
pub mod error;
pub mod fs;
pub mod judger;
pub mod logger;
pub mod schema;
pub mod server;

/// Create the Axum application with defined routes.
pub fn app() -> Router {
    Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/judge/problem/{pid}", get(get_problem_by_id))
        .route("/judge/problem/{pid}", delete(delete_problem_by_id))
        .route("/judge/submission", post(judge_problem_by_id))
        .route("/judge/problem/edit", patch(edit_problem_by_id))
        .route("/judge/problem/data/{pid}", post(upload_problem_data))
        .route("/judge/problem/config/{pid}", get(get_problem_config))
        .route("/judge/problem/config/{pid}", put(put_problem_config))
        .route("/judge/problem/tree/{pid}", get(get_problem_tree))
        .route(
            "/judge/problem/file/{pid}/{filename}",
            get(get_problem_file),
        )
        .route("/judge/submission/tree/{sid}", get(get_submission_tree))
        .route(
            "/judge/submission/file/{sid}/{filename}",
            get(get_submission_file),
        )
        .route(
            "/judge/problem/file/{pid}/{filename}",
            delete(delete_problem_file),
        )
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100 MB
        .layer(TraceLayer::new_for_http())
}

/// initialize the application (configuration, logger, etc.)
pub async fn initialize() -> &'static AijConfig {
    static INIT_ONCE: OnceCell<()> = OnceCell::const_new();
    let config = AijConfig::get();

    INIT_ONCE
        .get_or_init(|| async {
            logger::init_logger(&config.log_dir);
            config.update_binary_path().await;
        })
        .await;
    config
}
