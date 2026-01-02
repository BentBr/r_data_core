use thiserror::Error;

/// Specific authentication error kinds for better error handling
#[derive(Error, Debug, Clone)]
pub enum AuthErrorKind {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token validation failed: {0}")]
    TokenValidation(String),

    #[error("Token generation failed: {0}")]
    TokenGeneration(String),

    #[error("Account is not active")]
    AccountInactive,

    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Authentication error: {0}")]
    AuthError(#[from] AuthErrorKind),

    #[error("Authorization error: {0}")]
    Forbidden(String),

    #[error("Entity error: {0}")]
    Entity(String),

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

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::Unknown(err)
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Self::Unknown(err.to_string())
    }
}

impl From<uuid::Error> for Error {
    fn from(err: uuid::Error) -> Self {
        Self::Conversion(err.to_string())
    }
}

impl From<redis::RedisError> for Error {
    fn from(err: redis::RedisError) -> Self {
        Self::Cache(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// Extension trait to convert sqlx errors to our database errors
pub trait SqlxErrorExt: Sized {
    /// Convert an error into a database error
    ///
    /// # Errors
    /// Returns the original error if conversion fails
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
    fn test_auth_error_kind_display() {
        assert_eq!(
            AuthErrorKind::InvalidCredentials.to_string(),
            "Invalid credentials"
        );
        assert_eq!(AuthErrorKind::TokenExpired.to_string(), "Token expired");
        assert_eq!(
            AuthErrorKind::AccountInactive.to_string(),
            "Account is not active"
        );
        assert_eq!(
            AuthErrorKind::TokenValidation("bad token".to_string()).to_string(),
            "Token validation failed: bad token"
        );
        assert_eq!(
            AuthErrorKind::TokenGeneration("key error".to_string()).to_string(),
            "Token generation failed: key error"
        );
        assert_eq!(
            AuthErrorKind::Other("custom".to_string()).to_string(),
            "custom"
        );
    }

    #[test]
    fn test_auth_error_kind_clone() {
        let err = AuthErrorKind::InvalidCredentials;
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }

    #[test]
    fn test_error_from_auth_error_kind() {
        let auth_err = AuthErrorKind::TokenExpired;
        let err: Error = auth_err.into();
        assert!(matches!(err, Error::AuthError(AuthErrorKind::TokenExpired)));
    }

    #[test]
    fn test_error_from_string() {
        let err: Error = "test error".to_string().into();
        assert!(matches!(err, Error::Unknown(_)));
        assert_eq!(err.to_string(), "Unknown error: test error");
    }

    #[test]
    fn test_error_from_str() {
        let err: Error = "test".into();
        assert!(matches!(err, Error::Unknown(_)));
    }

    #[test]
    fn test_error_variants_display() {
        assert_eq!(
            Error::NotFound("item".to_string()).to_string(),
            "Not found: item"
        );
        assert_eq!(
            Error::Validation("invalid".to_string()).to_string(),
            "Validation error: invalid"
        );
        assert_eq!(
            Error::Forbidden("denied".to_string()).to_string(),
            "Authorization error: denied"
        );
    }

    #[test]
    fn test_error_from_uuid_error() {
        // Test that UUID parsing errors are converted to Conversion error
        let invalid_uuid = "not-a-valid-uuid";
        let uuid_err = uuid::Uuid::parse_str(invalid_uuid).unwrap_err();
        let err: Error = uuid_err.into();
        assert!(matches!(err, Error::Conversion(_)));
        assert!(err.to_string().contains("Conversion error"));
    }

    #[test]
    fn test_error_from_redis_error() {
        // Test that Redis errors are converted to Cache error
        use redis::RedisError;
        let redis_err = RedisError::from((redis::ErrorKind::IoError, "Connection failed"));
        let err: Error = redis_err.into();
        assert!(matches!(err, Error::Cache(_)));
        assert!(err.to_string().contains("Cache error"));
    }

    #[test]
    fn test_error_from_serde_json_error() {
        // Test that serde_json errors are converted to Serialization error
        let invalid_json = "not valid json {";
        let json_err = serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Serialization(_)));
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_all_error_variants() {
        // Test that all error variants can be created and displayed
        let errors = vec![
            Error::Database(sqlx::Error::PoolClosed),
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "file not found",
            )),
            Error::Auth("authentication failed".to_string()),
            Error::AuthError(AuthErrorKind::InvalidCredentials),
            Error::Forbidden("access denied".to_string()),
            Error::Entity("entity error".to_string()),
            Error::Config("config error".to_string()),
            Error::Api("api error".to_string()),
            Error::Cache("cache error".to_string()),
            Error::Deserialization("deserialization error".to_string()),
            Error::NotFound("not found".to_string()),
            Error::Validation("validation error".to_string()),
            Error::FieldNotFound("field not found".to_string()),
            Error::FieldAlreadyExists("field exists".to_string()),
            Error::ClassAlreadyExists("class exists".to_string()),
            Error::ValidationFailed("validation failed".to_string()),
            Error::FieldConversion("field".to_string(), "conversion error".to_string()),
            Error::Conversion("conversion error".to_string()),
            Error::ReadOnlyField("readonly field".to_string()),
            Error::PasswordHash("password hash error".to_string()),
            Error::Unknown("unknown error".to_string()),
            Error::InvalidSchema("invalid schema".to_string()),
            Error::InvalidFieldType("invalid field type".to_string()),
        ];

        // Verify all errors can be converted to string
        for err in errors {
            let _ = err.to_string();
        }
    }
}
