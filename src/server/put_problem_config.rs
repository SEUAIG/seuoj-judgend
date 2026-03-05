use crate::error::Result;
use crate::fs;
use crate::schema::{ProblemConfig, ProblemMetadata};
use crate::server::get_config_source::ConfigQuery;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use std::collections::HashSet;
use tracing::info;

pub(crate) async fn put_problem_config(
    Path(pid): Path<String>,
    Query(query): Query<ConfigQuery>,
    body: String,
) -> Result<impl IntoResponse> {
    let r#type = query.r#type.to_uppercase();
    info!(
        "Received request to update config for problem ID '{}', type '{}'",
        pid, r#type
    );

    fs::assert_problem_exists(&pid).await?;

    match r#type.as_str() {
        "META" => {
            let metadata = ProblemMetadata::from_json_str(&body)?;
            metadata.save(&pid).await?;
        }
        "INFO" => {
            let problem_case = ProblemConfig::from_toml_str(&body)?;
            let mut existing_id = HashSet::new();
            for (id, config) in &problem_case.testcases {
                if !existing_id.insert(id.clone()) {
                    return Err(crate::error::AijError::Request(
                        axum::http::StatusCode::BAD_REQUEST,
                        "DUPLICATE_TEST_CASE_ID".to_string(),
                        format!("Duplicate test case ID: {}", id),
                    ));
                }
                let in_path = &config.in_path;
                fs::validate_filename(in_path)?;
                let _ = fs::get_path_by_id_name(&pid, in_path, true).await?;
                let ans_path = &config.ans_path;
                if !ans_path.is_empty() {
                    fs::validate_filename(ans_path)?;
                    let _ = fs::get_path_by_id_name(&pid, ans_path, true).await?;
                }
            }
            problem_case.save(&pid).await?;
        }
        _ => {
            return Err(crate::error::AijError::Request(
                axum::http::StatusCode::BAD_REQUEST,
                "INVALID_TYPE".to_string(),
                format!("Invalid type: {}. Expected 'META' or 'CASE'", r#type),
            ));
        }
    }

    info!(
        "Successfully updated config for problem ID '{}', type '{}'",
        pid, r#type
    );
    Ok(Json(json!({
        "code": 0,
        "message": "Success",
    })))
}
