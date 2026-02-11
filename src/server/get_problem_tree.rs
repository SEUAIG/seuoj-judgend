use crate::error::{AijError, Result};
use crate::fs;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use tracing::{error, info};

use serde::Serialize;
use serde_json::json;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum FileNode {
    Directory {
        name: String,
        children: Vec<FileNode>,
    },
    File {
        name: String,
    },
}

pub(crate) async fn get_problem_tree(
    Path(pid): Path<String>,
) -> Result<impl IntoResponse> {
    info!("Received request to get problem tree for problem ID: {}", pid);

    fs::assert_problem_exists(&pid).await?;

    let problem_path = fs::get_dir_by_problem_id(&pid, false).await?;
    match build_tree(&problem_path) {
        Some(tree) => {
            info!("Successfully built file tree for problem ID: {}", pid);
            Ok(Json(json!({
                "code": 0,
                "message": "Success",
                "data": {
                    "tree": tree
                }
            })))
        }
        None => {
            let message = format!("Failed to build file tree for problem ID: {}", pid);
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