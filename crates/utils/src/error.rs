use thiserror::Error;

/// Custom error type for the application
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("File operation error: {0}")]
    FileOperation(String),

    #[error("Hash computation error: {0}")]
    Hash(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Image processing error: {0}")]
    Image(String),

    #[error("Video processing error: {0}")]
    Video(String),

    #[error("Task execution error: {0}")]
    Task(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Custom result type
pub type Result<T> = std::result::Result<T, Error>;

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::FileOperation("Test error".to_string());
        assert_eq!(err.to_string(), "File operation error: Test error");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }
}
