use crate::error::Result;
use crate::fs;
use crate::judger::{ProblemCase, ProblemInfo};
use crate::server::get_config_source::ConfigQuery;
use axum::Json;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use serde_json::json;
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
