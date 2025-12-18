use crate::error::Result;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
static INSTANCE: OnceLock<FileSystem> = OnceLock::new();

/// Read and write files to the local filesystem.
pub(crate) struct FileSystem {
    base_path: PathBuf,
}

impl FileSystem {
    /// Get the singleton instance of the FileSystem.
    pub(crate) fn get() -> Result<&'static Self> {
        INSTANCE.get().ok_or_else(|| {
            crate::error::AijError::FileSystem("FileSystem is not initialized".to_string())
        })
    }

    /// Initialize the FileSystem with the given base path.
    pub(crate) fn init(base_path: impl AsRef<Path>) -> Result<()> {
        // Create the path if it doesn't exist
        std::fs::create_dir_all(base_path.as_ref()).map_err(|e| {
            crate::error::AijError::FileSystem(format!(
                "Failed to create base path {}: {}",
                base_path.as_ref().to_string_lossy(),
                e
            ))
        })?;

        let fs = FileSystem {
            base_path: base_path.as_ref().to_path_buf(),
        };
        INSTANCE.set(fs).map_err(|_| {
            crate::error::AijError::FileSystem("FileSystem already initialized".to_string())
        })
    }
}

pub(crate) async fn read_problem_by_id(pid: &str) -> Result<String> {
    read_file_by_id_name(pid, "problem.md").await
}

pub(crate) async fn read_file_by_id_name(pid: &str, filename: &str) -> Result<String> {
    let file_path = get_path_by_id_name(pid, filename).await?;
    let content = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
        crate::error::AijError::FileSystem(format!(
            "Failed to read file {}: {}",
            file_path.to_string_lossy(),
            e
        ))
    })?;
    Ok(content)
}

pub(crate) async fn get_path_by_id_name(pid: &str, filename: &str) -> Result<PathBuf> {
    let fs = FileSystem::get()?;
    let file_path = fs.base_path.join(pid).join(filename);
    if !file_path.exists() {
        return Err(crate::error::AijError::FileSystem(format!(
            "File does not exist: {}",
            file_path.to_string_lossy()
        )));
    }
    Ok(file_path)
}

pub(crate) async fn get_dir_by_submission_id(submission_id: &str) -> Result<PathBuf> {
    let fs = FileSystem::get()?;
    let dir_path = fs.base_path.join("submissions").join(submission_id);
    if !dir_path.exists() {
        tokio::fs::create_dir_all(&dir_path).await.map_err(|e| {
            crate::error::AijError::FileSystem(format!(
                "Failed to create directories for {}: {}",
                dir_path.to_string_lossy(),
                e
            ))
        })?;
    }
    Ok(dir_path)
}
