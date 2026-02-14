use crate::error::Result;
use crate::fs;
use crate::judger::{ProblemCase, ProblemInfo};
use crate::server::get_config_source::ConfigQuery;
use axum::Json;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
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
            let problem_info = ProblemInfo::from_toml_str(&body)?;
            problem_info.save(&pid).await?;
        }
        "CASE" => {
            let problem_case = ProblemCase::from_toml_str(&body)?;
            let mut existing_id = HashSet::new();
            for case in &problem_case.test_cases {
                if !existing_id.insert(&case.id) {
                    return Err(crate::error::AijError::Request(
                        axum::http::StatusCode::BAD_REQUEST,
                        "DUPLICATE_TEST_CASE_ID".to_string(),
                        format!("Duplicate test case ID: {}", case.id),
                    ));
                }
                fs::validate_filename(&case.in_name)?;
                let _ =
                    fs::get_path_by_id_name(&pid, format!("data/{}", case.in_name), true).await?;
                if let Some(ans_name) = &case.ans_name {
                    fs::validate_filename(ans_name)?;
                    let _ =
                        fs::get_path_by_id_name(&pid, format!("data/{}", ans_name), true).await?;
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
