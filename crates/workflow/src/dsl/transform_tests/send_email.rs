use super::safe_field;
use crate::dsl::transform::{validate_transform, SendEmailTransform, StringOperand, Transform};

#[test]
fn valid_send_email_transform() {
    let t = Transform::SendEmail(SendEmailTransform {
        template_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        to: vec![StringOperand::Field {
            field: "user.email".to_string(),
        }],
        cc: None,
        target_status: "email_status".to_string(),
    });
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn send_email_empty_to_fails() {
    let t = Transform::SendEmail(SendEmailTransform {
        template_uuid: "some-uuid".to_string(),
        to: vec![],
        cc: None,
        target_status: "status".to_string(),
    });
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn send_email_unsafe_target_status_fails() {
    let t = Transform::SendEmail(SendEmailTransform {
        template_uuid: "some-uuid".to_string(),
        to: vec![StringOperand::ConstString {
            value: "a@b.com".to_string(),
        }],
        cc: None,
        target_status: "bad field!".to_string(),
    });
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn send_email_empty_template_uuid_fails() {
    let t = Transform::SendEmail(SendEmailTransform {
        template_uuid: "  ".to_string(),
        to: vec![StringOperand::ConstString {
            value: "a@b.com".to_string(),
        }],
        cc: None,
        target_status: "status".to_string(),
    });
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn send_email_with_const_recipients() {
    let t = Transform::SendEmail(SendEmailTransform {
        template_uuid: "uuid-123".to_string(),
        to: vec![StringOperand::ConstString {
            value: "admin@example.com".to_string(),
        }],
        cc: Some(vec![StringOperand::ConstString {
            value: "cc@example.com".to_string(),
        }]),
        target_status: "result".to_string(),
    });
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn send_email_unsafe_field_in_to_fails() {
    let t = Transform::SendEmail(SendEmailTransform {
        template_uuid: "uuid-123".to_string(),
        to: vec![StringOperand::Field {
            field: "bad field!".to_string(),
        }],
        cc: None,
        target_status: "status".to_string(),
    });
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn send_email_unsafe_field_in_cc_fails() {
    let t = Transform::SendEmail(SendEmailTransform {
        template_uuid: "uuid-123".to_string(),
        to: vec![StringOperand::ConstString {
            value: "a@b.com".to_string(),
        }],
        cc: Some(vec![StringOperand::Field {
            field: "bad!field".to_string(),
        }]),
        target_status: "status".to_string(),
    });
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn send_email_mixed_operand_types() {
    let t = Transform::SendEmail(SendEmailTransform {
        template_uuid: "uuid-123".to_string(),
        to: vec![
            StringOperand::Field {
                field: "user.email".to_string(),
            },
            StringOperand::ConstString {
                value: "admin@example.com".to_string(),
            },
        ],
        cc: Some(vec![StringOperand::ConstString {
            value: "cc@test.com".to_string(),
        }]),
        target_status: "email_result".to_string(),
    });
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}
