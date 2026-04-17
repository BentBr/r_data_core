#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::LazyLock;

use regex::Regex;

/// Pragmatic email validation pattern.
/// Checks: non-empty local part, @, non-empty domain, dot, non-empty TLD.
pub const EMAIL_PATTERN: &str = r"^[^\s@]+@[^\s@]+\.[^\s@]+$";

/// Compiled email regex for use with the `validator` crate.
/// Use as: `#[validate(regex(path = *EMAIL_RE))]`
pub static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(EMAIL_PATTERN).expect("Invalid email regex"));
