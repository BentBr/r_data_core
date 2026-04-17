#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

/// Minimum username length (characters)
pub const USERNAME_MIN_LENGTH: u64 = 3;
/// Maximum username length (characters)
pub const USERNAME_MAX_LENGTH: u64 = 50;
/// Minimum password length (characters)
pub const PASSWORD_MIN_LENGTH: u64 = 8;
/// Minimum length for first/last name fields
pub const NAME_MIN_LENGTH: u64 = 1;
/// Minimum length for API key name
pub const API_KEY_NAME_MIN_LENGTH: u64 = 1;
/// CSV delimiter/escape/quote must be exactly this many characters
pub const CSV_DELIMITER_LENGTH: u64 = 1;
/// DSL steps array minimum count
pub const DSL_STEPS_MIN_COUNT: u64 = 1;
