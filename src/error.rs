use actix_web::ResponseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Authorization error: {0}")]
    Forbidden(String),

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

    #[error("Field already exists: {0}")]
    FieldAlreadyExists(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Field conversion error for {0}: {1}")]
    FieldConversion(String, String),

    #[error("Conversion error: {0}")]
    Conversion(String),

    #[error("Read-only field: {0}")]
    ReadOnlyField(String),

    #[error("Password hashing error: {0}")]
    PasswordHash(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    #[error("Invalid field type: {0}")]
    InvalidFieldType(String),
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

impl ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// Extension trait to convert sqlx errors to our database errors
pub trait SqlxErrorExt: Sized {
    fn into_db_error(self) -> std::result::Result<sqlx::Error, Self>;
}

impl SqlxErrorExt for sqlx::Error {
    fn into_db_error(self) -> std::result::Result<sqlx::Error, Self> {
        Ok(self)
    }
}
