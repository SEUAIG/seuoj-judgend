use crate::error::AijError;
use crate::error::Result;
use axum::BoxError;
use axum::body::{Body, Bytes};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures_util::TryStream;
use serde::Serialize;

pub(crate) fn build_response_from_file_content<S>(
    pid: impl AsRef<str>,
    filename: impl AsRef<str>,
    file_content: S,
) -> Result<impl IntoResponse>
where
    S: TryStream + Send + 'static,
    S::Ok: Into<Bytes>,
    S::Error: Into<BoxError>,
{
    Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename.as_ref()),
        )
        .body(Body::from_stream(file_content))
        .map_err(|e| {
            AijError::Server(
                StatusCode::INTERNAL_SERVER_ERROR,
                "RESPONSE_BUILD_FAILED".to_string(),
                format!(
                    "Failed to build response for problem file {} of id {}: {}",
                    filename.as_ref(),
                    pid.as_ref(),
                    e
                ),
            )
        })
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub(crate) enum FileNode {
    Directory {
        name: String,
        children: Vec<FileNode>,
    },
    File {
        name: String,
    },
}
