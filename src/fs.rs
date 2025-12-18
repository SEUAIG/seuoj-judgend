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
            crate::error::AijError::FileSystemError("FileSystem is not initialized".to_string())
        })
    }

    /// Initialize the FileSystem with the given base path.
    pub(crate) fn init(base_path: impl AsRef<Path>) -> Result<()> {
        // Create the path if it doesn't exist
        std::fs::create_dir_all(base_path.as_ref()).map_err(|e| {
            crate::error::AijError::FileSystemError(format!(
                "Failed to create base path {}: {}",
                base_path.as_ref().to_string_lossy(),
                e
            ))
        })?;

        let fs = FileSystem {
            base_path: base_path.as_ref().to_path_buf(),
        };
        INSTANCE
            .set(fs)
            .map_err(|_| crate::error::AijError::FileSystemError("FileSystem already initialized".to_string()))
    }
}

pub(crate) async fn read_problem_by_id(pid: &str) -> Result<String> {
    let fs = FileSystem::get()?;
    let problem_path = fs.base_path.join(pid).join("problem.md");
    let content = tokio::fs::read_to_string(&problem_path).await.map_err(|e| {
        crate::error::AijError::FileSystemError(format!(
            "Failed to read problem file {}: {}",
            problem_path.to_string_lossy(),
            e
        ))
    })?;
    Ok(content)
}