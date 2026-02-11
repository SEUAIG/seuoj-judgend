use crate::error::Result;
use crate::fs;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info};


#[derive(Debug, Deserialize)]
pub(crate) struct ConfigQuery {
    r#type: String,
}


pub(crate) async fn get_config_source(
    Path(pid): Path<String>,
    Query(query): Query<ConfigQuery>,
) -> Result<impl IntoResponse> {
    let r#type = query.r#type.to_uppercase();
    info!("Getting config source for type: {} and problem id: {}", &r#type, &pid);

    if !fs::check_problem_exists(&pid).await? {
        error!("Problem with id '{}' not found", pid);
        return Err(crate::error::AijError::Request(
            StatusCode::NOT_FOUND,
            "PROBLEM_NOT_FOUND".to_string(),
            format!("Problem with id '{}' not found", pid),
        ));
    }

    match r#type.as_str() {
        "META" => {
            let content = fs::read_file_by_id_name(&pid, "info.toml").await?;
            Ok(Json(json!({ "code": 0, "message": "Success", "data": {"config": content} })))
        }
        "CASE" => {
            let content = fs::read_file_by_id_name(&pid, "data/case.toml").await.unwrap_or_default();
            Ok(Json(json!({ "code": 0, "message": "Success", "data": {"config": content} })))
        }
        _ => {
            Err(crate::error::AijError::Request(
                StatusCode::BAD_REQUEST,
                "INVALID_TYPE".to_string(),
                format!("Invalid type: {}. Expected 'META' or 'CASE'", r#type),
            ))
        }
    }
}