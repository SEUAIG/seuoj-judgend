//! Server-related functionalities for the AI Judge system.

mod edit_problem_by_id;
mod get_problem_by_id;
mod judge_problem_by_id;
mod upload_problem_data;

use crate::config::AijConfig;
use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use serde_json::json;
use std::sync::OnceLock;
use tokio::sync::Semaphore;
use tracing::error;

pub(crate) use edit_problem_by_id::edit_problem_by_id;
pub(crate) use get_problem_by_id::get_problem_by_id;
pub(crate) use judge_problem_by_id::judge_problem_by_id;
pub(crate) use upload_problem_data::upload_problem_data;

/// Custom extractor for JSON with custom error handling
pub(crate) struct AppJson<T>(pub T);

impl<S, T> FromRequest<S> for AppJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => {
                let message = format!("JSON extraction error: {}", rejection);
                error!("{}", message);
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "code": -1,
                        "message": message,
                    })),
                ))
            }
        }
    }
}

static JUDGE_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();

pub(crate) async fn get_judge_semaphore() -> &'static Semaphore {
    if JUDGE_SEMAPHORE.get().is_none() {
        let max_concurrent = AijConfig::get().max_concurrent_requests;
        let _ = JUDGE_SEMAPHORE.set(Semaphore::new(max_concurrent));
    }
    #[allow(clippy::unwrap_used)]
    JUDGE_SEMAPHORE.get().unwrap()
}
