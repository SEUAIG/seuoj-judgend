// Shared utility helpers for permission, compilation, and subtask topology.
use crate::config::AijConfig;
use crate::error::AijError;
use crate::error::Result;
use crate::schema::SubtaskConfig;
use reqwest::StatusCode;
use std::collections::HashMap;
use std::path::Path;
use tracing::error;

/// Make the file at `path` executable by adding execute permissions for user, group, and others.
pub(crate) async fn chmod_plus_x(path: impl AsRef<Path>) -> Result<()> {
    #[cfg(unix)]
    {
        let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FAILED_GET_FILE_METADATA".to_string(),
                format!(
                    "Failed to get metadata for file {}: {}",
                    path.as_ref().display(),
                    e
                ),
            )
        })?;
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            permissions.set_mode(permissions.mode() | 0o111);
            return tokio::fs::set_permissions(&path, permissions)
                .await
                .map_err(|e| {
                    AijError::FileSystem(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "FAILED_SET_FILE_PERMISSIONS".to_string(),
                        format!(
                            "Failed to set execute permissions for file {}: {}",
                            path.as_ref().display(),
                            e
                        ),
                    )
                });
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) async fn compile(path: impl AsRef<Path>) -> Result<()> {
    let path_without_ext = path.as_ref().with_extension("");
    if path_without_ext.exists() {
        return Ok(());
    }
    let testlib_path = &AijConfig::get().testlib_dir;
    let output = tokio::process::Command::new("g++")
        .arg(path.as_ref())
        .arg("-o")
        .arg(path.as_ref().with_extension(""))
        .arg("-O2")
        .arg("-static")
        .arg("-std=c++23")
        .arg("-I")
        .arg(testlib_path)
        .output()
        .await
        .map_err(|e| {
            let message = if e.kind() == std::io::ErrorKind::NotFound {
                "g++ not found. Please ensure g++ is installed and in the system PATH.".to_string()
            } else {
                e.to_string()
            };
            error!("Compilation error: {}", message);
            AijError::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                "COMPILATION_EXECUTION_FAILED".to_string(),
                format!(
                    "Failed to execute g++ for {}: {}",
                    path.as_ref().display(),
                    message
                ),
            )
        })?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        error!(
            "Compilation failed for {}: {}",
            path.as_ref().display(),
            message
        );
        return Err(AijError::Request(
            StatusCode::BAD_REQUEST,
            "COMPILATION_FAILED".to_string(),
            format!(
                "Compilation failed for {}: {}",
                path.as_ref().display(),
                message
            ),
        ));
    }
    Ok(())
}

pub(crate) fn get_topo_order(subtasks: &[SubtaskConfig]) -> Result<Vec<i32>> {
    let mut subtask_graph = HashMap::new();
    for subtask in subtasks {
        for pre_id in &subtask.pre_subtasks {
            subtask_graph
                .entry(*pre_id)
                .or_insert_with(Vec::new)
                .push(subtask.id);
        }
    }
    let mut topo_order = Vec::new();
    let mut in_degree = HashMap::new();
    for subtask in subtasks {
        in_degree.insert(subtask.id, subtask.pre_subtasks.len());
    }

    let mut queue = std::collections::VecDeque::new();
    for subtask in subtasks {
        if subtask.pre_subtasks.is_empty() {
            queue.push_back(subtask.id);
        }
    }

    while let Some(u) = queue.pop_front() {
        topo_order.push(u);
        if let Some(neighbors) = subtask_graph.get(&u) {
            for &v in neighbors {
                if let Some(d) = in_degree.get_mut(&v) {
                    *d -= 1;
                    if *d == 0 {
                        queue.push_back(v);
                    }
                }
            }
        }
    }

    if topo_order.len() != subtasks.len() {
        return Err(AijError::Request(
            StatusCode::BAD_REQUEST,
            "CIRCULAR_DEPENDENCY".to_string(),
            "Circular dependency detected among subtasks".to_string(),
        ));
    }

    Ok(topo_order)
}
