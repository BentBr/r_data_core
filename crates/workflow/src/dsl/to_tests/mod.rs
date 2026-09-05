#![allow(clippy::unwrap_used)]

mod auth_config;
mod entity_next_step;
mod format_output;
mod mapping_of_helper;

use super::*;
use regex::Regex;

pub(super) fn safe_field() -> Regex {
    Regex::new(r"^[A-Za-z_][A-Za-z0-9_.]*$").unwrap()
}

#[test]
fn valid_email_to() {
    let to_def = ToDef::Email {
        template_uuid: "uuid-123".to_string(),
        to: vec![super::super::transform::StringOperand::Field {
            field: "email".to_string(),
        }],
        cc: None,
        mapping: std::collections::HashMap::from([("name".to_string(), "user_name".to_string())]),
    };
    assert!(validate_to(0, &to_def, &safe_field()).is_ok());
}

#[test]
fn email_to_empty_recipients_fails() {
    let to_def = ToDef::Email {
        template_uuid: "uuid-123".to_string(),
        to: vec![],
        cc: None,
        mapping: std::collections::HashMap::new(),
    };
    assert!(validate_to(0, &to_def, &safe_field()).is_err());
}

#[test]
fn email_to_empty_template_uuid_fails() {
    let to_def = ToDef::Email {
        template_uuid: String::new(),
        to: vec![super::super::transform::StringOperand::ConstString {
            value: "a@b.com".to_string(),
        }],
        cc: None,
        mapping: std::collections::HashMap::new(),
    };
    assert!(validate_to(0, &to_def, &safe_field()).is_err());
}

#[test]
fn email_unsafe_field_in_to_fails() {
    let to_def = ToDef::Email {
        template_uuid: "uuid-123".to_string(),
        to: vec![super::super::transform::StringOperand::Field {
            field: "bad field!".to_string(),
        }],
        cc: None,
        mapping: std::collections::HashMap::new(),
    };
    assert!(validate_to(0, &to_def, &safe_field()).is_err());
}

#[test]
fn email_unsafe_field_in_cc_fails() {
    let to_def = ToDef::Email {
        template_uuid: "uuid-123".to_string(),
        to: vec![super::super::transform::StringOperand::ConstString {
            value: "a@b.com".to_string(),
        }],
        cc: Some(vec![super::super::transform::StringOperand::Field {
            field: "bad!field".to_string(),
        }]),
        mapping: std::collections::HashMap::new(),
    };
    assert!(validate_to(0, &to_def, &safe_field()).is_err());
}

#[test]
fn email_valid_with_cc_and_const_operands_ok() {
    let to_def = ToDef::Email {
        template_uuid: "uuid-123".to_string(),
        to: vec![super::super::transform::StringOperand::ConstString {
            value: "a@b.com".to_string(),
        }],
        cc: Some(vec![super::super::transform::StringOperand::ConstString {
            value: "cc@b.com".to_string(),
        }]),
        mapping: std::collections::HashMap::new(),
    };
    assert!(validate_to(0, &to_def, &safe_field()).is_ok());
}

#[test]
fn email_unsafe_mapping_destination_fails() {
    let to_def = ToDef::Email {
        template_uuid: "uuid-123".to_string(),
        to: vec![super::super::transform::StringOperand::ConstString {
            value: "a@b.com".to_string(),
        }],
        cc: None,
        mapping: std::collections::HashMap::from([(
            "bad field!".to_string(),
            "source".to_string(),
        )]),
    };
    assert!(validate_to(0, &to_def, &safe_field()).is_err());
}
