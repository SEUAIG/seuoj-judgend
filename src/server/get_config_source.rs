use crate::error::Result;
use crate::fs;
use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;
use tracing::info;

#[derive(Debug, Deserialize)]
pub(crate) struct ConfigQuery {
    pub(crate) r#type: String,
}

pub(crate) async fn get_config_source(
    Path(pid): Path<String>,
    Query(query): Query<ConfigQuery>,
) -> Result<impl IntoResponse> {
    let r#type = query.r#type.to_uppercase();
    info!(
        "Getting config source for type: {} and problem id: {}",
        &r#type, &pid
    );

    fs::assert_problem_exists(&pid).await?;

    match r#type.as_str() {
        "META" => {
            let content = fs::read_file_by_id_name(&pid, "problem.json").await?;
            Ok(Json(
                json!({ "code": 0, "message": "Success", "data": {"config": content} }),
            ))
        }
        "INFO" => {
            let content = fs::read_file_by_id_name(&pid, "info.toml")
                .await
                .unwrap_or_default();
            Ok(Json(
                json!({ "code": 0, "message": "Success", "data": {"config": content} }),
            ))
        }
        _ => Err(crate::error::AijError::Request(
            StatusCode::BAD_REQUEST,
            "INVALID_TYPE".to_string(),
            format!("Invalid type: {}. Expected 'META' or 'INFO'", r#type),
        )),
    }
}
