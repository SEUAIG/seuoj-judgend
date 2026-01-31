//! File system operations for reading and writing problem and submission data.
use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::judger::ProblemInfo;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use tokio::sync::OnceCell;
use tokio_util::io::ReaderStream;

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
                            AijError::FileSystem(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "CREATE_PROBLEMS_DIR_FAILED".to_string(),
                                format!(
                                    "Failed to create problems directory {}: {}",
                                    config.problems_dir.to_string_lossy(),
                                    e
                                ),
                            )
                        })?;
                }
                Ok(FileSystem {
                    base_path: config.problems_dir.clone(),
                })
            })
            .await
    }
}

/// Test case input/output pair for a problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Case {
    pub(crate) id: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) r#in: Option<String>,
    pub(crate) in_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ans: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ans_name: Option<String>,
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

pub(crate) async fn read_problem_by_id(pid: impl AsRef<str>) -> Result<Problem> {
    let problem_info = ProblemInfo::from_pid(&pid).await?;
    let mut example = vec![];
    let mut sample_index = 1;
    while let Ok(r#in) = read_file_by_id_name(&pid, &format!("example_{}.in", sample_index)).await {
        let ans = read_file_by_id_name(&pid, &format!("example_{}.ans", sample_index))
            .await
            .unwrap_or_default();
        let description = read_file_by_id_name(&pid, &format!("example_{}.md", sample_index))
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
        pid: pid.as_ref().to_string(),
        description: read_file_by_id_name(&pid, "description.md").await?,
        input: read_file_by_id_name(&pid, "input.md").await?,
        output: read_file_by_id_name(&pid, "output.md")
            .await
            .unwrap_or_default(),
        example,
        info: problem_info,
    })
}

pub(crate) async fn read_file_by_id_name(
    pid: impl AsRef<str>,
    filename: impl AsRef<str>,
) -> Result<String> {
    let file_path = get_path_by_id_name(pid, filename, true).await?;
    get_text_by_path(file_path, None).await
}

pub(crate) async fn get_path_by_id_name(
    pid: impl AsRef<str>,
    filename: impl AsRef<str>,
    check: bool,
) -> Result<PathBuf> {
    let problem_dir = get_dir_by_problem_id(pid, false).await?;
    let file_path = problem_dir.join(filename.as_ref());
    if check && !file_path.exists() {
        return Err(AijError::FileSystem(
            StatusCode::NOT_FOUND,
            "FILE_NOT_FOUND".to_string(),
            format!("File does not exist: {}", file_path.to_string_lossy()),
        ));
    }
    Ok(file_path)
}

pub(crate) async fn get_dir_by_problem_id(pid: impl AsRef<str>, create: bool) -> Result<PathBuf> {
    let fs = FileSystem::get().await?;
    let dir_path = fs.base_path.join(pid.as_ref());
    if create && !dir_path.exists() {
        create_dir_all(&dir_path).await?;
    }
    Ok(dir_path)
}

pub(crate) async fn get_dir_by_submission_id(submission_id: impl AsRef<str>) -> Result<PathBuf> {
    let fs = FileSystem::get().await?;
    let dir_path = fs
        .base_path
        .join("submissions")
        .join(submission_id.as_ref());
    if !dir_path.exists() {
        create_dir_all(&dir_path).await?;
    }
    Ok(dir_path)
}

pub(crate) async fn delete_dir_by_submission_id(submission_id: impl AsRef<str>) -> Result<()> {
    let dir_path = get_dir_by_submission_id(submission_id).await?;
    if dir_path.exists() {
        tokio::fs::remove_dir_all(&dir_path).await.map_err(|e| {
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "REMOVE_SUBMISSION_DIR_FAILED".to_string(),
                format!(
                    "Failed to remove directory {}: {}",
                    dir_path.to_string_lossy(),
                    e
                ),
            )
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
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "OPEN_FILE_FAILED".to_string(),
                format!(
                    "Failed to open file {}: {}",
                    path.as_ref().to_string_lossy(),
                    e
                ),
            )
        })?;
        let mut buffer = vec![0u8; len];
        let n = file.read(&mut buffer).await.map_err(|e| {
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "READ_FILE_FAILED".to_string(),
                format!("Read error: {}", e),
            )
        })?;
        Ok(String::from_utf8_lossy(&buffer[..n]).into_owned())
    } else {
        tokio::fs::read_to_string(path.as_ref()).await.map_err(|e| {
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "OPEN_FILE_FAILED".to_string(),
                format!(
                    "Failed to read file {}: {}",
                    path.as_ref().to_string_lossy(),
                    e
                ),
            )
        })
    }
}

pub(crate) async fn get_stream_by_path(
    path: impl AsRef<Path>,
) -> Result<ReaderStream<tokio::fs::File>> {
    let file = tokio::fs::File::open(path.as_ref()).await.map_err(|e| {
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "OPEN_FILE_FAILED".to_string(),
            format!(
                "Failed to open file {}: {}",
                path.as_ref().to_string_lossy(),
                e
            ),
        )
    })?;
    Ok(ReaderStream::new(file))
}

pub(crate) async fn create_dir_all(path: impl AsRef<Path>) -> Result<()> {
    tokio::fs::create_dir_all(path.as_ref()).await.map_err(|e| {
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create directories".to_string(),
            format!(
                "Failed to create directories for {}: {}",
                path.as_ref().to_string_lossy(),
                e
            ),
        )
    })
}

pub(crate) async fn write_to_file(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> Result<()> {
    if let Some(parent) = path.as_ref().parent()
        && !parent.exists()
    {
        create_dir_all(parent).await?;
    }
    tokio::fs::write(path.as_ref(), content).await.map_err(|e| {
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "WRITE_FILE_FAILED".to_string(),
            format!(
                "Failed to write to file {}: {}",
                path.as_ref().to_string_lossy(),
                e
            ),
        )
    })
}

pub(crate) async fn remove_file(path: impl AsRef<Path>) -> Result<()> {
    if !path.as_ref().exists() {
        return Ok(());
    }
    tokio::fs::remove_file(path.as_ref()).await.map_err(|e| {
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "REMOVE_FILE_FAILED".to_string(),
            format!(
                "Failed to remove file {}: {}",
                path.as_ref().to_string_lossy(),
                e
            ),
        )
    })
}

pub(crate) async fn check_problem_exists(pid: impl AsRef<str>) -> Result<bool> {
    let problem_dir = get_dir_by_problem_id(pid, false).await?;
    Ok(problem_dir.exists())
}

pub(crate) fn validate_filename(filename: impl AsRef<str>) -> Result<()> {
    let filename = filename.as_ref();
    let regex = regex::Regex::new(r"^[a-zA-Z0-9_-]+(\.[a-zA-Z0-9_-]+)*$").map_err(|e| {
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "REGEX_COMPILE_FAILED".to_string(),
            format!("Failed to compile regex: {}", e),
        )
    })?;
    if filename.is_empty() || !regex.is_match(filename) {
        return Err(AijError::Request(
            StatusCode::BAD_REQUEST,
            "INVALID_FILENAME".to_string(),
            format!("Invalid filename: {filename}"),
        ));
    }
    Ok(())
}
