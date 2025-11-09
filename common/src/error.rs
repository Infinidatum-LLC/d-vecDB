use thiserror::Error;

pub type Result<T> = std::result::Result<T, VectorDbError>;

#[derive(Error, Debug)]
pub enum VectorDbError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    #[error("Vector not found: {id}")]
    VectorNotFound { id: String },

    #[error("Collection not found: {name}")]
    CollectionNotFound { name: String },

    #[error("Collection already exists: {name}")]
    CollectionAlreadyExists { name: String },

    #[error("Invalid distance metric: {metric}")]
    InvalidDistanceMetric { metric: String },

    #[error("Index error: {message}")]
    IndexError { message: String },

    #[error("Storage error: {message}")]
    StorageError { message: String },

    #[error("Network error: {message}")]
    NetworkError { message: String },

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    #[error("Not found: {message}")]
    NotFound { message: String },

    #[error("Data corruption: {message}")]
    Corruption { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl From<serde_json::Error> for VectorDbError {
    fn from(err: serde_json::Error) -> Self {
        VectorDbError::Serialization(err.to_string())
    }
}