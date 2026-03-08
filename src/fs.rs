//! File system operations for reading and writing problem and submission data.

use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::schema::ProblemMetadata;
use axum::http::StatusCode;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::io::AsyncReadExt;
use tokio::sync::OnceCell;
use tokio_util::io::ReaderStream;
use tracing::warn;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

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
                            warn!("Failed to create problems directory: {}", e);
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

pub(crate) async fn read_problem_by_id(pid: impl AsRef<str>) -> Result<ProblemMetadata> {
    ProblemMetadata::from_pid(pid).await
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
        warn!("File not found: {}", file_path.to_string_lossy());
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
            let message = format!(
                "Failed to remove submission directory {}: {}",
                dir_path.to_string_lossy(),
                e
            );
            warn!("{}", message);
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "REMOVE_SUBMISSION_DIR_FAILED".to_string(),
                message,
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
            let message = format!(
                "Failed to open file {}: {}",
                path.as_ref().to_string_lossy(),
                e
            );
            warn!("{}", message);
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "OPEN_FILE_FAILED".to_string(),
                message,
            )
        })?;
        let mut buffer = vec![0u8; len];
        let n = file.read(&mut buffer).await.map_err(|e| {
            warn!(
                "Failed to read file {}: {}",
                path.as_ref().to_string_lossy(),
                e
            );
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "READ_FILE_FAILED".to_string(),
                format!("Read error: {}", e),
            )
        })?;
        Ok(String::from_utf8_lossy(&buffer[..n]).into_owned())
    } else {
        tokio::fs::read_to_string(path.as_ref()).await.map_err(|e| {
            warn!(
                "Failed to read file {}: {}",
                path.as_ref().to_string_lossy(),
                e
            );
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
        warn!(
            "Failed to open file {}: {}",
            path.as_ref().to_string_lossy(),
            e
        );
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
        warn!(
            "Failed to create directories {}: {}",
            path.as_ref().to_string_lossy(),
            e
        );
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
        warn!(
            "Failed to write to file {}: {}",
            path.as_ref().to_string_lossy(),
            e
        );
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

pub(crate) async fn check_problem_exists(pid: impl AsRef<str>) -> Result<bool> {
    let problem_dir = get_dir_by_problem_id(pid, false).await?;
    Ok(problem_dir.exists())
}

pub(crate) async fn assert_problem_exists(pid: impl AsRef<str>) -> Result<()> {
    if !check_problem_exists(&pid).await? {
        warn!("Problem not found: {}", pid.as_ref());
        return Err(AijError::FileSystem(
            StatusCode::NOT_FOUND,
            "PROBLEM_NOT_FOUND".to_string(),
            format!("Problem with ID {} does not exist", pid.as_ref()),
        ));
    }
    Ok(())
}

pub(crate) fn validate_filename(filename: impl AsRef<str>) -> Result<()> {
    let filename = filename.as_ref();

    static FILENAME_REGEX: OnceLock<regex::Regex> = OnceLock::new();
    let regex = FILENAME_REGEX.get_or_init(|| {
        // SAFE: HARD-CODED REGEX, NO USER INPUT
        #[allow(clippy::expect_used)]
        regex::Regex::new(r"^[a-zA-Z0-9_-]+(\.[a-zA-Z0-9_-]+)*$")
            .expect("Failed to compile filename validation regex")
    });
    if filename.is_empty() || !regex.is_match(filename) {
        warn!("Invalid filename: {}", filename);
        return Err(AijError::Request(
            StatusCode::BAD_REQUEST,
            "INVALID_FILENAME".to_string(),
            format!("Invalid filename: {filename}"),
        ));
    }
    Ok(())
}

pub(crate) async fn unzip_bytes_to_path(
    bytes: impl AsRef<[u8]> + Send,
    path: impl AsRef<Path> + Send,
) -> Result<()> {
    let path_buf = path.as_ref().to_path_buf();
    let bytes_vec = bytes.as_ref().to_vec();

    tokio::task::spawn_blocking(move || -> std::result::Result<(), String> {
        let reader = std::io::Cursor::new(bytes_vec);
        let mut zip = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;

        for i in 0..zip.len() {
            let mut file = zip.by_index(i).map_err(|e| e.to_string())?;

            validate_filename(file.name()).map_err(|e| e.to_string())?;

            let out_path = path_buf.join(file.mangled_name());

            if file.is_dir() {
                std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }

                let mut out_file = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;

                std::io::copy(&mut file, &mut out_file).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| {
        warn!("Failed to join blocking task: {}", e);
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "TASK_JOIN_FAILED".to_string(),
            format!("Blocking task failed: {}", e),
        )
    })?
    .map_err(|e| {
        warn!("Failed to unzip file: {}", e);
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "UNZIP_FAILED".to_string(),
            format!("Failed to unzip file: {}", e),
        )
    })
}

pub(crate) async fn remove_dir_all(path: impl AsRef<Path>) -> Result<()> {
    if path.as_ref().exists() {
        tokio::fs::remove_dir_all(path.as_ref())
            .await
            .map_err(|e| {
                warn!(
                    "Failed to remove directory {}: {}",
                    path.as_ref().to_string_lossy(),
                    e
                );
                AijError::FileSystem(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "REMOVE_DIR_FAILED".to_string(),
                    format!(
                        "Failed to remove directory {}: {}",
                        path.as_ref().to_string_lossy(),
                        e
                    ),
                )
            })?;
    }
    Ok(())
}

pub(crate) async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    tokio::fs::rename(from.as_ref(), to.as_ref())
        .await
        .map_err(|e| {
            let message = format!(
                "Failed to rename from {} to {}: {}",
                from.as_ref().to_string_lossy(),
                to.as_ref().to_string_lossy(),
                e
            );
            warn!("{}", message);
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "RENAME_FAILED".to_string(),
                message,
            )
        })
}

pub(crate) async fn delete_file(path: impl AsRef<Path>) -> Result<()> {
    if path.as_ref().exists() {
        tokio::fs::remove_file(path.as_ref()).await.map_err(|e| {
            warn!(
                "Failed to delete file {}: {}",
                path.as_ref().to_string_lossy(),
                e
            );
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DELETE_FILE_FAILED".to_string(),
                format!(
                    "Failed to delete file {}: {}",
                    path.as_ref().to_string_lossy(),
                    e
                ),
            )
        })?;
    }
    Ok(())
}

pub(crate) async fn create_zip_file(
    paths: &[PathBuf],
    output_path: impl AsRef<Path>,
) -> Result<()> {
    let output_path_buf = output_path.as_ref().to_path_buf();
    let paths = paths.to_vec();
    tokio::task::spawn_blocking(move || -> std::result::Result<(), String> {
        let file = std::fs::File::create(&output_path_buf).map_err(|e| e.to_string())?;
        let mut zip = ZipWriter::new(file);

        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for path in paths {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                zip.start_file(file_name, options)
                    .map_err(|e| e.to_string())?;

                let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
                zip.write_all(&buffer).map_err(|e| e.to_string())?;
            }
        }

        zip.finish().map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| {
        warn!("Failed to join blocking task: {}", e);
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "TASK_JOIN_FAILED".to_string(),
            format!("Blocking task failed: {}", e),
        )
    })?
    .map_err(|e| {
        warn!("Failed to create zip file: {}", e);
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ZIP_CREATION_FAILED".to_string(),
            format!("Failed to create zip file: {}", e),
        )
    })
}

pub(crate) async fn get_data_paths_by_id(pid: impl AsRef<str>) -> Result<Vec<PathBuf>> {
    let data_dir = get_dir_by_problem_id(pid, false).await?.join("data");
    let mut data_files = Vec::new();
    if data_dir.exists() {
        let mut entries = tokio::fs::read_dir(data_dir).await.map_err(|e| {
            warn!("Failed to read data directory: {}", e);
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "READ_DATA_DIR_FAILED".to_string(),
                format!("Failed to read data directory: {}", e),
            )
        })?;
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            warn!("Failed to read data directory: {}", e);
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "READ_DATA_DIR_FAILED".to_string(),
                format!("Failed to read data directory: {}", e),
            )
        })? {
            let path = entry.path();
            if path.is_file() {
                data_files.push(path);
            }
        }
    }
    Ok(data_files)
}
