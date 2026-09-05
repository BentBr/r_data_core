#![allow(clippy::unwrap_used)]

mod arithmetic_concat;
mod authenticate;
mod entity_path;
mod send_email;

use super::*;
use regex::Regex;

pub(super) fn safe_field() -> Regex {
    Regex::new(r"^[A-Za-z_][A-Za-z0-9_.]*$").unwrap()
}

#[test]
fn none_transform_always_validates_ok() {
    assert!(validate_transform(0, &Transform::None, &safe_field()).is_ok());
}
