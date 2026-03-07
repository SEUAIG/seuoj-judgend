use crate::error::Result;
use crate::fs;
use crate::schema::{
    CheckerType, ProblemConfig, ProblemType,
};
use crate::server::AppJson;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use std::collections::HashSet;
use tracing::info;

pub(crate) async fn put_problem_config(
    Path(pid): Path<String>,
    AppJson(problem_config): AppJson<ProblemConfig>,
) -> Result<impl IntoResponse> {
    info!("Received request to update config for problem ID '{}'", pid);

    fs::assert_problem_exists(&pid).await?;

    match problem_config.problem_info.problem_type {
        ProblemType::Interactive => {
            if problem_config.problem_info.checker_type == CheckerType::Interactor
                && let Some(custom_modules) = &problem_config.custom_modules
                && let Some(interactor_path) = &custom_modules.interactor_path
            {
                fs::validate_filename(interactor_path)?;
                let interactor_full_path =
                    fs::get_path_by_id_name(&pid, format!("data/{}", interactor_path), true)
                        .await?;
                if interactor_path.contains(".cpp") {
                    crate::judger::compile(interactor_full_path).await?;
                }
            } else {
                return Err(crate::error::AijError::Request(
                    axum::http::StatusCode::BAD_REQUEST,
                    "INVALID_CHECKER_TYPE".to_string(),
                    "Checker type must be 'Interactor' for interactive problems".to_string(),
                ));
            }
        }
        ProblemType::Special => {
            if problem_config.problem_info.checker_type == CheckerType::Special
                && let Some(custom_modules) = &problem_config.custom_modules
                && let Some(checker_path) = &custom_modules.checker_path
            {
                fs::validate_filename(checker_path)?;
                let checker_full_path =
                    fs::get_path_by_id_name(&pid, format!("data/{}", checker_path), true).await?;
                if checker_path.contains(".cpp") {
                    crate::judger::compile(checker_full_path).await?;
                }
            } else {
                return Err(crate::error::AijError::Request(
                    axum::http::StatusCode::BAD_REQUEST,
                    "INVALID_CHECKER_TYPE".to_string(),
                    "Checker type must be 'Custom' for special problems".to_string(),
                ));
            }
        }
        _ => {}
    }

    let mut existing_id = HashSet::new();
    for case in &problem_config.testcases {
        if !existing_id.insert(case.id) {
            return Err(crate::error::AijError::Request(
                axum::http::StatusCode::BAD_REQUEST,
                "DUPLICATE_TEST_CASE_ID".to_string(),
                format!("Duplicate test case ID: {}", case.id),
            ));
        }
        let in_path = &case.in_path;
        fs::validate_filename(in_path)?;
        let _ = fs::get_path_by_id_name(&pid, format!("data/{in_path}"), true).await?;
        let ans_path = &case.ans_path;
        if !ans_path.is_empty() {
            fs::validate_filename(ans_path)?;
            let _ = fs::get_path_by_id_name(&pid, format!("data/{ans_path}"), true).await?;
        }
    }
    problem_config.save(&pid).await?;

    info!("Successfully updated config for problem ID '{}'", pid);
    Ok(Json(json!({
        "code": 0,
        "message": "Success",
    })))
}
