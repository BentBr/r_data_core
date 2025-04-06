use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Entity error: {0}")]
    Entity(String),

    #[error("Workflow error: {0}")]
    Workflow(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Field not found: {0}")]
    FieldNotFound(String),
    
    #[error("Field conversion error for {0}: {1}")]
    FieldConversion(String, String),
    
    #[error("Read-only field: {0}")]
    ReadOnlyField(String),

    #[error("Password hashing error: {0}")]
    PasswordHash(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<redis::RedisError> for Error {
    fn from(err: redis::RedisError) -> Self {
        Error::Cache(err.to_string())
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Unknown(err)
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Error::Unknown(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>; 