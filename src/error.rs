pub(crate) type Result<T> = std::result::Result<T, AijError>;

/// An enumeration representing possible errors in the application.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum AijError {
    /// Configuration related errors
    ConfigError(String),
    /// Logging related errors
    LoggerError(String),
}