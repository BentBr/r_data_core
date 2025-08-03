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

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Field not found: {0}")]
    FieldNotFound(String),

    #[error("Field already exists: {0}")]
    FieldAlreadyExists(String),

    #[error("Class already exists: {0}")]
    ClassAlreadyExists(String),

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

impl From<uuid::Error> for Error {
    fn from(err: uuid::Error) -> Self {
        Error::Conversion(err.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_message_formatting() {
        let error = Error::Database(sqlx::Error::RowNotFound);

        assert_eq!(
            error.to_string(),
            "Database error: no rows returned by a query that expected to return at least one row"
        );
    }

    #[test]
    fn test_not_found_message() {
        let error = Error::NotFound("User with id '123' not found".to_string());

        assert_eq!(error.to_string(), "Not found: User with id '123' not found");
    }

    #[test]
    fn test_validation_error_message() {
        let error = Error::Validation("Email format is invalid".to_string());

        assert_eq!(
            error.to_string(),
            "Validation error: Email format is invalid"
        );
    }

    #[test]
    fn test_api_error_message() {
        let error = Error::Api("Invalid request".to_string());

        assert_eq!(error.to_string(), "API error: Invalid request");
    }

    #[test]
    fn test_auth_error_message() {
        let error = Error::Auth("Invalid credentials".to_string());

        assert_eq!(
            error.to_string(),
            "Authentication error: Invalid credentials"
        );
    }

    #[test]
    fn test_deserialization_error_message() {
        let error =
            Error::Deserialization("invalid type: string \"abc\", expected i64".to_string());

        assert_eq!(
            error.to_string(),
            "Deserialization error: invalid type: string \"abc\", expected i64"
        );
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error: Error = io_error.into();

        assert!(matches!(error, Error::Io(_)));
    }
}
