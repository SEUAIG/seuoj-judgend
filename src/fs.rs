//! File system operations for reading and writing problem and submission data.
use crate::config::AijConfig;
use crate::error::Result;
use crate::judger::ProblemInfo;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use tokio::sync::OnceCell;

static INSTANCE: OnceCell<FileSystem> = OnceCell::const_new();

/// Read and write files to the local filesystem.
pub struct FileSystem {
    base_path: PathBuf,
}

impl FileSystem {
    /// Get the singleton instance of the FileSystem.
    pub(crate) async fn get() -> Result<&'static Self> {
        INSTANCE
            .get_or_try_init(async || {
                let config = AijConfig::get();
                if !config.problems_dir.exists() {
                    tokio::fs::create_dir_all(&config.problems_dir)
                        .await
                        .map_err(|e| {
                            crate::error::AijError::FileSystem(format!(
                                "Failed to create problems directory {}: {}",
                                config.problems_dir.to_string_lossy(),
                                e
                            ))
                        })?;
                }
                Ok(FileSystem {
                    base_path: config.problems_dir.clone(),
                })
            })
            .await
    }
}

/// Sample input/output pair for a problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Sample {
    pub(crate) r#in: String,
    pub(crate) ans: String,
    pub(crate) description: String,
}

/// Problem metadata and description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Problem {
    pub(crate) pid: String,
    pub(crate) description: String,
    pub(crate) input: String,
    pub(crate) output: String,
    pub(crate) example: Vec<Sample>,
    pub(crate) info: ProblemInfo,
}

pub(crate) async fn read_problem_by_id(pid: &str) -> Result<Problem> {
    let problem_info = ProblemInfo::from_pid(pid).await?;
    if problem_info.test_case_number.is_none() {
        return Err(crate::error::AijError::FileSystem(format!(
            "Problem {} is missing test case number info",
            pid
        )));
    }
    let mut example = vec![];
    let mut sample_index = 1;
    while let Ok(r#in) = read_file_by_id_name(pid, &format!("example_{}.in", sample_index)).await {
        let ans = read_file_by_id_name(pid, &format!("example_{}.ans", sample_index))
            .await
            .unwrap_or_default();
        let description = read_file_by_id_name(pid, &format!("example_{}.md", sample_index))
            .await
            .unwrap_or_default();
        example.push(Sample {
            r#in,
            ans,
            description,
        });
        sample_index += 1;
    }
    Ok(Problem {
        pid: pid.to_string(),
        description: read_file_by_id_name(pid, "description.md").await?,
        input: read_file_by_id_name(pid, "input.md").await?,
        output: read_file_by_id_name(pid, "output.md")
            .await
            .unwrap_or_default(),
        example,
        info: problem_info,
    })
}

pub(crate) async fn read_file_by_id_name(pid: &str, filename: &str) -> Result<String> {
    let file_path = get_path_by_id_name(pid, filename, true).await?;
    get_text_by_path(file_path, None).await
}

pub(crate) async fn get_path_by_id_name(pid: &str, filename: &str, check: bool) -> Result<PathBuf> {
    let problem_dir = get_dir_by_problem_id(pid, !check).await?;
    let file_path = problem_dir.join(filename);
    if check && !file_path.exists() {
        return Err(crate::error::AijError::FileSystem(format!(
            "File does not exist: {}",
            file_path.to_string_lossy()
        )));
    }
    Ok(file_path)
}

pub(crate) async fn get_dir_by_problem_id(pid: &str, create: bool) -> Result<PathBuf> {
    let fs = FileSystem::get().await?;
    let dir_path = fs.base_path.join(pid);
    if create && !dir_path.exists() {
        create_dir_all(&dir_path).await?;
    }
    Ok(dir_path)
}

pub(crate) async fn get_dir_by_submission_id(submission_id: &str) -> Result<PathBuf> {
    let fs = FileSystem::get().await?;
    let dir_path = fs.base_path.join("submissions").join(submission_id);
    if !dir_path.exists() {
        create_dir_all(&dir_path).await?;
    }
    Ok(dir_path)
}

pub(crate) async fn delete_dir_by_submission_id(submission_id: &str) -> Result<()> {
    let dir_path = get_dir_by_submission_id(submission_id).await?;
    if dir_path.exists() {
        tokio::fs::remove_dir_all(&dir_path).await.map_err(|e| {
            crate::error::AijError::FileSystem(format!(
                "Failed to remove directory {}: {}",
                dir_path.to_string_lossy(),
                e
            ))
        })?;
    }
    Ok(())
}

pub(crate) async fn get_text_by_path(
    path: impl AsRef<Path>,
    truncate_len: Option<usize>,
) -> Result<String> {
    if let Some(len) = truncate_len {
        let mut file = tokio::fs::File::open(&path).await.map_err(|e| {
            crate::error::AijError::FileSystem(format!(
                "Failed to open file {}: {}",
                path.as_ref().to_string_lossy(),
                e
            ))
        })?;
        let mut buffer = vec![0u8; len];
        let n = file
            .read(&mut buffer)
            .await
            .map_err(|e| crate::error::AijError::FileSystem(format!("Read error: {}", e)))?;
        Ok(String::from_utf8_lossy(&buffer[..n]).into_owned())
    } else {
        tokio::fs::read_to_string(path.as_ref()).await.map_err(|e| {
            crate::error::AijError::FileSystem(format!(
                "Failed to read file {}: {}",
                path.as_ref().to_string_lossy(),
                e
            ))
        })
    }
}

pub(crate) async fn create_dir_all(path: impl AsRef<Path>) -> Result<()> {
    tokio::fs::create_dir_all(path.as_ref()).await.map_err(|e| {
        crate::error::AijError::FileSystem(format!(
            "Failed to create directories for {}: {}",
            path.as_ref().to_string_lossy(),
            e
        ))
    })
}

pub(crate) async fn write_to_file(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> Result<()> {
    tokio::fs::write(path.as_ref(), content).await.map_err(|e| {
        crate::error::AijError::FileSystem(format!(
            "Failed to write to file {}: {}",
            path.as_ref().to_string_lossy(),
            e
        ))
    })
}

pub(crate) async fn remove_file(path: impl AsRef<Path>) -> Result<()> {
    tokio::fs::remove_file(path.as_ref()).await.map_err(|e| {
        crate::error::AijError::FileSystem(format!(
            "Failed to remove file {}: {}",
            path.as_ref().to_string_lossy(),
            e
        ))
    })
}