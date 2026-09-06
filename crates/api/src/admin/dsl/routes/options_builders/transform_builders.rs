#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use super::path_transform_builders::{
    build_build_path_transform_fields, build_get_or_create_entity_transform_fields,
    build_resolve_entity_path_transform_fields,
};
use crate::admin::dsl::models::{DslFieldSpec, DslTypeSpec};

/// Build field specifications for arithmetic transform type
pub(super) fn build_arithmetic_transform_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "target".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "left.kind".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec![
                "field".into(),
                "const".into(),
                "external_entity_field".into(),
            ]),
        },
        DslFieldSpec {
            name: "left.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "left.value".into(),
            r#type: "number".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "left.entity_definition".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "left.filter.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "left.filter.value".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "op".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec!["add".into(), "sub".into(), "mul".into(), "div".into()]),
        },
        DslFieldSpec {
            name: "right.kind".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec![
                "field".into(),
                "const".into(),
                "external_entity_field".into(),
            ]),
        },
        DslFieldSpec {
            name: "right.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "right.value".into(),
            r#type: "number".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "right.entity_definition".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "right.filter.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "right.filter.value".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
    ]
}

/// Build field specifications for concat transform type
pub(super) fn build_concat_transform_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "target".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "separator".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "fields".into(),
            r#type: "array<string>".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build field specifications for the authenticate transform type
pub(super) fn build_authenticate_transform_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "entity_type".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "identifier_field".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "password_field".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "input_identifier".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "input_password".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "target_token".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "extra_claims".into(),
            r#type: "object<string, string>".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "token_expiry_seconds".into(),
            r#type: "number".into(),
            required: false,
            options: None,
        },
    ]
}

/// Build field specifications for the `send_email` transform type
pub(super) fn build_send_email_transform_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "template_uuid".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "to".into(),
            r#type: "array<string>".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "cc".into(),
            r#type: "array<string>".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "target_status".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build transform type specifications
#[must_use]
pub fn build_transform_type_specs(workflow_mail_configured: bool) -> Vec<DslTypeSpec> {
    let mut specs = vec![
        DslTypeSpec {
            r#type: "none".to_string(),
            fields: vec![],
        },
        DslTypeSpec {
            r#type: "arithmetic".to_string(),
            fields: build_arithmetic_transform_fields(),
        },
        DslTypeSpec {
            r#type: "concat".to_string(),
            fields: build_concat_transform_fields(),
        },
        DslTypeSpec {
            r#type: "resolve_entity_path".to_string(),
            fields: build_resolve_entity_path_transform_fields(),
        },
        DslTypeSpec {
            r#type: "build_path".to_string(),
            fields: build_build_path_transform_fields(),
        },
        DslTypeSpec {
            r#type: "get_or_create_entity".to_string(),
            fields: build_get_or_create_entity_transform_fields(),
        },
        DslTypeSpec {
            r#type: "authenticate".to_string(),
            fields: build_authenticate_transform_fields(),
        },
    ];
    if workflow_mail_configured {
        specs.push(DslTypeSpec {
            r#type: "send_email".to_string(),
            fields: build_send_email_transform_fields(),
        });
    }
    specs
}
