use crate::error::{AijError, Result};
use crate::fs;
use axum::Json;
use axum::extract::{Multipart, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use tracing::{error, info, warn};

/// Maximum compressed archive size: 200 MiB, excluding multipart overhead.
pub(crate) const MAX_TESTCASE_ARCHIVE_BYTES: usize = 200 * 1024 * 1024;
/// One extra MiB allows multipart headers and the format field at the file boundary.
pub(crate) const MAX_TESTCASE_UPLOAD_REQUEST_BYTES: usize =
    MAX_TESTCASE_ARCHIVE_BYTES + 1024 * 1024;

pub(crate) async fn upload_problem_data(
    Path(pid): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse> {
    info!("Uploading test cases for problem id: {}", &pid);

    fs::assert_problem_exists(&pid).await?;

    let mut file = None;
    let mut format = None;

    while let Some(part) = multipart.next_field().await.map_err(|e| {
        error!("Failed to read multipart field: {}", e);
        AijError::Request(
            e.status(),
            "MULTIPART_READ_ERROR".to_string(),
            if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
                "压缩包大小不得超过 200 MiB".to_string()
            } else {
                format!("Failed to read multipart field: {}", e)
            },
        )
    })? {
        let name = part.name().unwrap_or_default();
        match name {
            "file" => {
                file = Some(part.bytes().await.map_err(|e| {
                    error!("Failed to read file field: {}", e);
                    AijError::Request(
                        e.status(),
                        "FILE_READ_ERROR".to_string(),
                        if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
                            "压缩包大小不得超过 200 MiB".to_string()
                        } else {
                            format!("Failed to read file field: {}", e)
                        },
                    )
                })?);
                if file
                    .as_ref()
                    .is_some_and(|bytes| bytes.len() > MAX_TESTCASE_ARCHIVE_BYTES)
                {
                    return Err(AijError::Request(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "ARCHIVE_TOO_LARGE".to_string(),
                        "压缩包大小不得超过 200 MiB".to_string(),
                    ));
                }
            }
            "format" => {
                format = Some(part.text().await.map_err(|e| {
                    error!("Failed to read format field: {}", e);
                    AijError::Request(
                        StatusCode::BAD_REQUEST,
                        "FORMAT_READ_ERROR".to_string(),
                        format!("Failed to read format field: {}", e),
                    )
                })?);
            }
            _ => {
                error!("Unexpected multipart field: {}", name);
                return Err(AijError::Request(
                    StatusCode::BAD_REQUEST,
                    "UNEXPECTED_FIELD".to_string(),
                    format!("Unexpected multipart field: {}", name),
                ));
            }
        }
    }
    if format.is_none() {
        format = Some("zip".to_string());
    }
    if let Some(file) = file
        && let Some(format) = format
    {
        match format.as_str() {
            "zip" => {
                let problem_dir = fs::get_dir_by_problem_id(&pid, false).await?;
                let tmp_dir = tempfile::tempdir_in(problem_dir).map_err(|e| {
                    warn!("Failed to create temporary directory: {}", e);
                    AijError::FileSystem(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "TEMP_DIR_CREATION_ERROR".to_string(),
                        format!("Failed to create temporary directory: {}", e),
                    )
                })?;
                info!(
                    "Created temporary directory to unzip at {:?}",
                    tmp_dir.path()
                );
                match fs::unzip_bytes_to_path(file, &tmp_dir).await {
                    Ok(_) => {
                        let data_path = fs::get_path_by_pid_name(&pid, "data/", false).await?;
                        fs::remove_dir_all(&data_path).await?;
                        fs::rename(tmp_dir, data_path).await?;
                    }
                    Err(e) => {
                        error!("Failed to unzip file for problem id: {}", &pid);
                        tmp_dir.close().ok();
                        return Err(e);
                    }
                }
            }
            _ => {
                error!("Unsupported format: {}", format);
                return Err(AijError::Request(
                    StatusCode::BAD_REQUEST,
                    "UNSUPPORTED_FORMAT".to_string(),
                    format!("Unsupported format: {}", format),
                ));
            }
        }

        info!("Successfully uploaded test cases for problem id: {}", &pid);

        Ok(Json(json!({
            "code": 0,
            "message": "Success",
        })))
    } else {
        error!("Missing file field");
        Err(AijError::Request(
            StatusCode::BAD_REQUEST,
            "MISSING_FIELD".to_string(),
            "Missing file field".to_string(),
        ))
    }
}
