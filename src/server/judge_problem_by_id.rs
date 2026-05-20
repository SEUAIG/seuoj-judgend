use crate::config::AijConfig;
use crate::error::AijError;
use crate::error::Result;
use crate::fs::{assert_problem_exists, delete_dir_by_submission_id};
use crate::judger::judge_online;
use crate::judger::{JudgeResult, SupportedLanguages, judge};
use crate::schema::OnlineCase;
use crate::schema::ProblemConfig;
use crate::server::{AppJson, get_judge_semaphore};
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use tracing::{debug, error, info, warn};

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
    #[serde(rename = "testcases")]
    #[serde(default)]
    pub(crate) testcases: Vec<OnlineCase>,
}

pub(crate) async fn judge_problem_by_id(
    AppJson(payload): AppJson<JudgeRequest>,
) -> Result<impl IntoResponse> {
    info!(
        "Received judge request: submission_id={}, problem_id={}, language={:?}",
        &payload.submission_id, &payload.problem_id, &payload.language
    );
    assert_problem_exists(&payload.problem_id)
        .await
        .map_err(|e| e.set_code(StatusCode::BAD_REQUEST))?;
    let problem_config = ProblemConfig::from_pid(&payload.problem_id).await?;
    if problem_config.testcases.is_empty() {
        let message = format!("Problem {} has no test cases.", &payload.problem_id);
        error!("{}", message);
        return Err(AijError::Request(
            StatusCode::UNPROCESSABLE_ENTITY,
            "NO_TEST_CASES".to_string(),
            message,
        ));
    }
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
            Err(AijError::Judge(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SEMAPHORE_ACQUIRE_FAILED".to_string(),
                message,
            ))
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
        let json_content = parse_content_from_result(res);
        debug!(
            "Result to report: {}, submission_id={}",
            json_content, payload.submission_id
        );
        info!(
            "Reporting result to backend for submission_id={}",
            payload.submission_id
        );
        let client = Client::new();
        match client.put(&server_addr).json(&json_content).send().await {
            Ok(resp) => {
                info!(
                    "Reported result to backend for submission_id={}: response_status={}",
                    payload.submission_id,
                    resp.status()
                );
            }
            Err(e) => {
                warn!(
                    "Failed to report result to backend {} for submission_id={}: {}",
                    server_addr, payload.submission_id, e
                );
            }
        }
    });
    Ok(Json(json!({
        "code": 0,
        "message": "Success",
    })))
}

pub(crate) async fn judge_problem_online_by_id(
    AppJson(payload): AppJson<JudgeRequest>,
) -> Result<impl IntoResponse> {
    info!(
        "Received online judge request: problem_id={}, language={:?}",
        &payload.problem_id, &payload.language
    );
    assert_problem_exists(&payload.problem_id)
        .await
        .map_err(|e| e.set_code(StatusCode::BAD_REQUEST))?;
    if payload.testcases.is_empty() {
        let message = format!("Problem {} has no test cases.", &payload.problem_id);
        error!("{}", message);
        return Err(AijError::Request(
            StatusCode::UNPROCESSABLE_ENTITY,
            "NO_TEST_CASES".to_string(),
            message,
        ));
    }
    let res = judge_online(
        payload.problem_id,
        payload.code,
        payload.language,
        payload.submission_id.clone(),
        payload.testcases,
    )
    .await;

    Ok(Json(json!({
        "code": 0,
        "message": "Success",
        "data": parse_content_from_result(res),
    })))
}

fn parse_content_from_result(res: Result<JudgeResult>) -> Value {
    match res {
        Ok(result) => {
            info!("Online judging completed: {:?}", result);
            match result {
                JudgeResult::CompileError { detail } => json!({
                    "status": "CompileError",
                    "errorDetail": detail,
                }),
                JudgeResult::CodeTooLong { detail } => json!({
                    "status": "CodeTooLong",
                    "errorDetail": detail,
                }),
                JudgeResult::MaybeError {
                    results,
                    subtask_configs,
                } => {
                    let score: i32 = if subtask_configs.is_empty() {
                        results.iter().map(|r| r.score).sum()
                    } else {
                        subtask_configs.iter().map(|s| s.score).sum()
                    };
                    json!({
                        "status": "Success",
                        "resultDetail": results,
                        "subtasks": subtask_configs,
                        "score": score,
                    })
                }
            }
        }
        Err(e) => json!({
            "status": "JudgendError",
            "errorDetail": format!("Judging failed: {}", e),
        }),
    }
}
