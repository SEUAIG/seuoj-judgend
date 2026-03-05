use crate::error::{AijError, Result};
use crate::fs;
use axum::extract::{Multipart, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use tracing::{error, info};

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
            StatusCode::BAD_REQUEST,
            "MULTIPART_READ_ERROR".to_string(),
            format!("Failed to read multipart field: {}", e),
        )
    })? {
        let name = part.name().unwrap_or_default();
        match name {
            "file" => {
                file = Some(part.bytes().await.map_err(|e| {
                    error!("Failed to read file field: {}", e);
                    AijError::Request(
                        StatusCode::BAD_REQUEST,
                        "FILE_READ_ERROR".to_string(),
                        format!("Failed to read file field: {}", e),
                    )
                })?);
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
                let tmp_path = fs::get_path_by_id_name(&pid, "tmpdata/", false).await?;
                match fs::unzip_bytes_to_path(file, &tmp_path).await {
                    Ok(_) => {
                        let data_path = fs::get_path_by_id_name(&pid, "data/", false).await?;
                        fs::remove_dir_all(&data_path).await?;
                        fs::rename(tmp_path, data_path).await?;
                    }
                    Err(e) => {
                        error!("Failed to unzip file for problem id: {}", &pid);
                        fs::remove_dir_all(&tmp_path).await?;
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
