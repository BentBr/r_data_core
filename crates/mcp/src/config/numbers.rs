#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Numeric environment variables.

use std::collections::HashMap;

use super::{get, ConfigError};

/// A positive integer variable, or its default when unset.
///
/// Shared by the timeout and the server limits so every numeric variable is
/// refused the same way. `0` and negatives are refused: none of the values
/// this reads is meaningful at zero.
pub fn positive_u64(
    map: &HashMap<String, String>,
    key: &'static str,
    default: u64,
) -> Result<u64, ConfigError> {
    get(map, key).map_or(Ok(default), |raw| match raw.parse::<u64>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(ConfigError::InvalidNumber(key, raw.to_string())),
    })
}
