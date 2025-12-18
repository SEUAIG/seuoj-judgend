use std::fmt::Display;

pub(crate) type Result<T> = std::result::Result<T, AijError>;

/// An enumeration representing possible errors in the application.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum AijError {
    /// Configuration related errors
    ConfigError(String),
    /// Logging related errors
    LoggerError(String),
    /// Server related errors
    ServerError(String),
    /// File system related errors
    FileSystemError(String),
}


impl Display for AijError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AijError::ConfigError(msg) => write!(f, "Configuration Error: {}", msg),
            AijError::LoggerError(msg) => write!(f, "Logger Error: {}", msg),
            AijError::ServerError(msg) => write!(f, "Server Error: {}", msg),
            AijError::FileSystemError(msg) => write!(f, "File System Error: {}", msg),
        }
    }
}