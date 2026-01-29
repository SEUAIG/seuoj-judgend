#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
//! SEU AIJ Judge-Endpoint

use crate::server::{edit_problem_by_id, get_problem_by_id, judge_problem_by_id, upload_problem_data};
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};
use axum::Router;

pub mod config;
pub mod error;
pub mod fs;
pub mod judger;
pub mod logger;
pub mod server;

/// Create the Axum application with defined routes.
pub fn app() -> Router {
    Router::new()
        .route("/judge/problem/{pid}", get(get_problem_by_id))
        .route("/judge/submission", post(judge_problem_by_id))
        .route("/judge/problem/edit", patch(edit_problem_by_id))
        .route("/judge/problem/data", post(upload_problem_data))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100 MB
}
