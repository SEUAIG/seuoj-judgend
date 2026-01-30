use crate::error::{AijError, Result};
use crate::fs;
use crate::fs::get_raw_by_path;
use axum::body::Body;
use axum::extract::Path;
use axum::http::header;
use axum::response::{IntoResponse, Response};

pub(crate) async fn serve_problem_file(
    Path((pid, filename)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let file_path = fs::get_path_by_id_name(&pid, &filename, true).await?;
    let file_content = get_raw_by_path(file_path).await?;
    Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(file_content))
        .map_err(|e| {
            AijError::Server(format!(
                "Failed to build response for problem file {} of problem {}: {}",
                filename, pid, e
            ))
        })
}
