use crate::error::AijError;
use crate::fs::read_problem_by_id;
use axum::Json;
use axum::extract::Path;
use axum::response::IntoResponse;
use serde_json::json;
use tracing::{info, warn};

pub(crate) async fn get_problem_by_id(Path(id): Path<String>) -> impl IntoResponse {
    info!("Received request for problem ID: {}", id);
    let content = read_problem_by_id(&id).await;
    match content {
        Ok(content) => {
            info!("Successfully retrieved problem ID: {}", id);
            Json(json!({
                "code": 0,
                "message": "Success",
                "data": content,
            }))
            .into_response()
        }
        Err(e) => {
            warn!("Error retrieving problem ID {}: {}", id, e);
            AijError::FileSystem("Problem not found".to_string()).into_response()
        }
    }
}
