use crate::error::Result;
use crate::fs;
use crate::schema::ProblemConfig;
use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use tracing::info;

pub(crate) async fn get_problem_config(Path(pid): Path<String>) -> Result<impl IntoResponse> {
    info!("Getting config source for problem id: {}", &pid);

    fs::assert_problem_exists(&pid).await?;

    let problem_config = ProblemConfig::from_pid(&pid).await?;
    Ok(Json(
        json!({ "code": 0, "message": "Success", "data": problem_config }),
    ))
}
