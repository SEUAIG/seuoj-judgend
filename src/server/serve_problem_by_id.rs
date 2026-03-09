use crate::error::Result;
use crate::fs;
use crate::fs::read_problem_by_id;
use axum::Json;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use tracing::info;

pub(crate) async fn get_problem_by_id(Path(pid): Path<String>) -> Result<impl IntoResponse> {
    info!("Received request for problem ID: {}", pid);
    let content = read_problem_by_id(&pid).await?;
    info!("Successfully retrieved problem ID: {}", pid);
    Ok(Json(json!({
        "code": 0,
        "message": "Success",
        "data": content,
    })))
}

pub(crate) async fn delete_problem_by_id(Path(pid): Path<String>) -> Result<impl IntoResponse> {
    info!("Received delete request for problem ID: {}", pid);
    let problem_path = fs::get_dir_by_problem_id(&pid, false).await?;
    fs::remove_dir_all(problem_path).await?;
    info!("Successfully deleted problem ID: {}", pid);
    Ok(StatusCode::NO_CONTENT)
}
