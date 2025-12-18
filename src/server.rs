use crate::fs::read_problem_by_id;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use tracing::info;

#[derive(Serialize)]
struct Problem {
    pid: String,
    content: String,
}

#[derive(Serialize)]
struct ProblemResponse {
    code: i32,
    message: String,
    data: Option<Problem>,
}
pub(crate) async fn get_problem_by_id(Path(id): Path<String>) -> impl IntoResponse {
    info!("Received request for problem ID: {}", id);
    let content = read_problem_by_id(&id).await;
    match content {
        Ok(problem_content) => {
            let response = ProblemResponse {
                code: 0,
                message: "Success".to_string(),
                data: Some(Problem {
                    pid: id,
                    content: problem_content,
                }),
            };
            Json(response).into_response()
        }
        Err(e) => {
            let status = StatusCode::NOT_FOUND;
            let response = ProblemResponse {
                code: -1,
                message: format!("Error retrieving problem: {}", e),
                data: None,
            };
            (status, Json(response)).into_response()
        }
    }
}