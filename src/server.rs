//! Server-related functionalities for the AI Judge system.
use crate::config::AijConfig;
use crate::fs::{delete_dir_by_submission_id, read_problem_by_id};
use crate::judger::{judge, JudgeResult, SupportedLanguages};
use axum::extract::rejection::JsonRejection;
use axum::extract::Path;
use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::sync::OnceLock;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

pub(crate) async fn get_problem_by_id(Path(id): Path<String>) -> impl IntoResponse {
    info!("Received request for problem ID: {}", id);
    let content = read_problem_by_id(&id).await;
    match content {
        Ok(content) => Json(json!({
            "code": 0,
            "message": "Success",
            "data": content,
        }))
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "code": -1,
                "message": format!("Error retrieving problem: {}", e),
            })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub(crate) struct JudgeRequest {
    #[serde(rename = "submissionId")]
    pub(crate) submission_id: String,
    #[serde(rename = "pid")]
    pub(crate) problem_id: String,
    #[serde(rename = "code")]
    pub(crate) code: String,
    #[serde(rename = "language")]
    pub(crate) language: SupportedLanguages,
}

pub(crate) async fn judge_problem_by_id(
    AppJson(payload): AppJson<JudgeRequest>,
) -> impl IntoResponse {
    info!(
        "Received judge request: submission_id={}, problem_id={}, language={:?}",
        &payload.submission_id, &payload.problem_id, &payload.language
    );
    tokio::spawn(async move {
        let semaphore = get_judge_semaphore().await;
        let sem = semaphore.acquire().await;
        let res = judge(
            payload.problem_id,
            payload.code,
            payload.language,
            payload.submission_id.clone(),
        )
            .await;
        drop(sem);
        match AijConfig::get().await {
            Ok(config) => {
                if !config.save_submissions
                    && let Err(e) = delete_dir_by_submission_id(&payload.submission_id).await
                {
                    warn!(
                        "Failed to delete submission files for submission_id={}: {}",
                        &payload.submission_id, e
                    );
                }
            }
            Err(e) => {
                error!("Failed to get config for logging: {}", e);
            }
        }
        let server_addr = match AijConfig::get_backend_base_addr().await {
            Ok(addr) => {
                format!("{}/judge/submission/{}", addr, &payload.submission_id)
            }
            Err(e) => {
                error!("Failed to get backend base address for reporting: {}", e);
                return;
            }
        };
        let json_content = match res {
            Ok(result) => {
                info!("Judging completed: {:?}", result);
                match result {
                    JudgeResult::CompileError(s) => json!({
                        "status": "CompileError",
                        "errorDetail": s,
                    }),
                    JudgeResult::MaybeError(vec) => json!({
                        "status": "Success",
                        "resultDetail": vec,
                    }),
                }
            }
            Err(e) => json!({
                "status": "JudgendError",
                "errorDetail": format!("Judging failed: {}", e),
            }),
        };
        info!(
            "Reporting result to backend for submission_id={}: {}",
            &payload.submission_id, json_content
        );
        let client = Client::new();
        match client.put(&server_addr).json(&json_content).send().await {
            Ok(resp) => {
                info!(
                    "Reported result to backend for submission_id={}: response_status={}",
                    &payload.submission_id,
                    resp.status()
                );
            }
            Err(e) => {
                warn!(
                    "Failed to report result to backend for submission_id={}: {}",
                    &payload.submission_id, e
                );
            }
        }
    });
    let response = json!({
        "code": 0,
        "message": "Success",
    });
    Json(response)
}

/// Custom extractor for JSON with custom error handling
pub(crate) struct AppJson<T>(pub T);

impl<S, T> FromRequest<S> for AppJson<T>
where
    Json<T>: FromRequest<S, Rejection=JsonRejection>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => {
                let response = json!({
                    "code": -1,
                    "message": format!("Invalid JSON: {}", rejection.body_text()),
                });
                Err((StatusCode::BAD_REQUEST, Json(response)))
            }
        }
    }
}

static JUDGE_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();

pub(crate) async fn get_judge_semaphore() -> &'static Semaphore {
    if JUDGE_SEMAPHORE.get().is_none() {
        let max_concurrent = match AijConfig::get().await {
            Ok(config) => config.max_concurrent_requests,
            Err(_) => 6,
        };
        let _ = JUDGE_SEMAPHORE.set(Semaphore::new(max_concurrent));
    }
    #[allow(clippy::unwrap_used)]
    JUDGE_SEMAPHORE.get().unwrap()
}
