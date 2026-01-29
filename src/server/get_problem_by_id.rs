use crate::error::Result;
use crate::fs::read_problem_by_id;
use axum::Json;
use axum::extract::Path;
use axum::response::IntoResponse;
use serde_json::json;
use tracing::info;

pub(crate) async fn get_problem_by_id(Path(id): Path<String>) -> Result<impl IntoResponse> {
    info!("Received request for problem ID: {}", id);
    let content = read_problem_by_id(&id).await?;
    info!("Successfully retrieved problem ID: {}", id);
    Ok(Json(json!({
        "code": 0,
        "message": "Success",
        "data": content,
    })))
}
