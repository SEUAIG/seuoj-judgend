use crate::config::AijConfig;
use crate::error::AijError::Judge;
use crate::fs::delete_dir_by_submission_id;
use crate::judger::{JudgeResult, SupportedLanguages, judge};
use crate::server::{AppJson, get_judge_semaphore};
use axum::Json;
use axum::response::IntoResponse;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info, warn};

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
        let res = if let Ok(permit) = sem {
            let res = judge(
                payload.problem_id,
                payload.code,
                payload.language,
                payload.submission_id.clone(),
            )
            .await;
            drop(permit);
            res
        } else {
            let message = format!(
                "Failed to acquire semaphore for submission_id={}",
                &payload.submission_id
            );
            error!("{}", message);
            Err(Judge(message))
        };

        if !AijConfig::get().save_submissions
            && let Err(e) = delete_dir_by_submission_id(&payload.submission_id).await
        {
            warn!(
                "Failed to delete submission files for submission_id={}: {}",
                &payload.submission_id, e
            );
        }

        let server_addr = match AijConfig::get_backend_base_addr() {
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
