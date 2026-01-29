//! Error handling module for the application.
use axum::response::{IntoResponse, Response};
use std::fmt::Display;

/// A specialized `Result` type for the application.
pub type Result<T> = std::result::Result<T, AijError>;

/// An enumeration representing possible errors in the application.
#[derive(Debug)]
#[allow(dead_code)]
pub enum AijError {
    /// Configuration related errors
    Config(String),
    /// Logging related errors
    Logger(String),
    /// Server related errors
    Server(String),
    /// File system related errors
    FileSystem(String),
    /// Judge related errors
    Judge(String),
    /// Request related errors
    Request(String),
}

impl Display for AijError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AijError::Config(msg) => write!(f, "Configuration Error: {}", msg),
            AijError::Logger(msg) => write!(f, "Logger Error: {}", msg),
            AijError::Server(msg) => write!(f, "Server Error: {}", msg),
            AijError::FileSystem(msg) => write!(f, "File System Error: {}", msg),
            AijError::Judge(msg) => write!(f, "Judge Error: {}", msg),
            AijError::Request(msg) => write!(f, "Request Error: {}", msg),
        }
    }
}

impl IntoResponse for AijError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AijError::Config(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            AijError::Logger(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            AijError::Server(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            AijError::FileSystem(msg) => (axum::http::StatusCode::NOT_FOUND, msg),
            AijError::Judge(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
            AijError::Request(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
        };
        let body = axum::Json(serde_json::json!({
            "code": -1,
            "message": message,
        }));
        (status, body).into_response()
    }
}
