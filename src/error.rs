//! Error handling module for the application.
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::fmt::Display;

/// A specialized `Result` type for the application.
pub type Result<T> = std::result::Result<T, AijError>;

/// An enumeration representing possible errors in the application.
#[derive(Debug)]
pub enum AijError {
    /// Configuration related errors
    Config(StatusCode, String, String),
    /// Logging related errors
    Logger(StatusCode, String, String),
    /// Server related errors
    Server(StatusCode, String, String),
    /// File system related errors
    FileSystem(StatusCode, String, String),
    /// Judge related errors
    Judge(StatusCode, String, String),
    /// Request related errors
    Request(StatusCode, String, String),
}

impl AijError {
    /// set error code
    pub fn set_code(self, code: StatusCode) -> Self {
        match self {
            AijError::Config(_, short, long) => AijError::Config(code, short, long),
            AijError::Logger(_, short, long) => AijError::Logger(code, short, long),
            AijError::Server(_, short, long) => AijError::Server(code, short, long),
            AijError::FileSystem(_, short, long) => AijError::FileSystem(code, short, long),
            AijError::Judge(_, short, long) => AijError::Judge(code, short, long),
            AijError::Request(_, short, long) => AijError::Request(code, short, long),
        }
    }
}

impl Display for AijError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AijError::Config(.., msg) => write!(f, "Configuration Error: {}", msg),
            AijError::Logger(.., msg) => write!(f, "Logger Error: {}", msg),
            AijError::Server(.., msg) => write!(f, "Server Error: {}", msg),
            AijError::FileSystem(.., msg) => write!(f, "File System Error: {}", msg),
            AijError::Judge(.., msg) => write!(f, "Judge Error: {}", msg),
            AijError::Request(.., msg) => write!(f, "Request Error: {}", msg),
        }
    }
}

impl IntoResponse for AijError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AijError::Config(code, short, long) => (code, format!("{}: {}", short, long)),
            AijError::Logger(code, short, long) => (code, format!("{}: {}", short, long)),
            AijError::Server(code, short, long) => (code, format!("{}: {}", short, long)),
            AijError::FileSystem(code, short, long) => (code, format!("{}: {}", short, long)),
            AijError::Judge(code, short, long) => (code, format!("{}: {}", short, long)),
            AijError::Request(code, short, long) => (code, format!("{}: {}", short, long)),
        };
        let body = axum::Json(serde_json::json!({
            "code": -1,
            "message": message,
        }));
        (status, body).into_response()
    }
}
