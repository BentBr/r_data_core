#[cfg(test)]
mod error_handling_tests {
    use r_data_core::error::Error;

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
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error: Error = io_error.into();

        assert!(matches!(error, Error::Io(_)));
    }
}
