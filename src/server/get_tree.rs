use crate::error::{AijError, Result};
use crate::fs;
use axum::Json;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tracing::{error, info};

use crate::server::utils::FileNode;
use serde_json::json;

pub(crate) async fn get_problem_tree(Path(pid): Path<String>) -> Result<impl IntoResponse> {
    info!(
        "Received request to get problem tree for problem ID: {}",
        pid
    );

    fs::assert_problem_exists(&pid).await?;

    let problem_path = fs::get_dir_by_problem_id(&pid, false).await?;
    get_response_tree(problem_path, pid).await
}

pub(crate) async fn get_submission_tree(Path(sid): Path<String>) -> Result<impl IntoResponse> {
    info!(
        "Received request to get submission tree for submission ID: {}",
        sid
    );

    fs::assert_submission_exists(&sid).await?;

    let submission_path = fs::get_dir_by_submission_id(&sid, false).await?;
    get_response_tree(submission_path, sid).await
}

async fn get_response_tree(
    path: impl AsRef<std::path::Path>,
    id: impl AsRef<str>,
) -> Result<impl IntoResponse> {
    match build_tree(path.as_ref()) {
        Some(tree) => {
            info!("Successfully built file tree for ID: {}", id.as_ref());
            Ok(Json(json!({
                "code": 0,
                "message": "Success",
                "data": {
                    "tree": tree
                }
            })))
        }
        None => {
            let message = format!("Failed to build file tree for ID: {}", id.as_ref());
            error!("{}", message);
            Err(AijError::Server(
                StatusCode::INTERNAL_SERVER_ERROR,
                "TREE_BUILD_FAILED".to_string(),
                message,
            ))
        }
    }
}

pub fn build_tree(path: &std::path::Path) -> Option<Vec<FileNode>> {
    if !path.is_dir() {
        return None;
    }

    let mut nodes = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();

            if name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                let children = build_tree(&path).unwrap_or_default();
                nodes.push(FileNode::Directory { name, children });
            } else {
                nodes.push(FileNode::File { name });
            }
        }
    }

    Some(nodes)
}
