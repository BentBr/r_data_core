#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::admin::dsl::models::{DslFieldSpec, DslTypeSpec};

/// Build field specifications for format TO type
pub(super) fn build_format_to_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "output.mode".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec!["api".into(), "download".into(), "push".into()]),
        },
        DslFieldSpec {
            name: "output.push.destination.destination_type".into(),
            r#type: "string".into(),
            required: false,
            options: Some(vec!["uri".into()]),
        },
        DslFieldSpec {
            name: "output.push.destination.config.uri".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "output.push.method".into(),
            r#type: "enum".into(),
            required: false,
            options: Some(vec![
                "GET".into(),
                "POST".into(),
                "PUT".into(),
                "PATCH".into(),
                "DELETE".into(),
                "HEAD".into(),
                "OPTIONS".into(),
            ]),
        },
        DslFieldSpec {
            name: "output.push.destination.auth".into(),
            r#type: "object".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.format_type".into(),
            r#type: "string".into(),
            required: true,
            options: Some(vec!["csv".into(), "json".into()]),
        },
        DslFieldSpec {
            name: "format.options.has_header".into(),
            r#type: "boolean".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.options.delimiter".into(),
            r#type: "string(1)".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.options.escape".into(),
            r#type: "string(1)".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.options.quote".into(),
            r#type: "string(1)".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "mapping".into(),
            r#type: "map<string,string>".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build field specifications for entity TO type
pub(super) fn build_entity_to_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "entity_definition".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "path".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "mode".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec!["create".into(), "update".into()]),
        },
        DslFieldSpec {
            name: "identify.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "identify.value".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "update_key".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "mapping".into(),
            r#type: "map<string,string>".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build field specifications for email TO type
pub(super) fn build_email_to_fields() -> Vec<DslFieldSpec> {
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
            name: "mapping".into(),
            r#type: "map<string,string>".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build TO type specifications
#[must_use]
pub fn build_to_type_specs(workflow_mail_configured: bool) -> Vec<DslTypeSpec> {
    let mut specs = vec![
        DslTypeSpec {
            r#type: "format".to_string(),
            fields: build_format_to_fields(),
        },
        DslTypeSpec {
            r#type: "entity".to_string(),
            fields: build_entity_to_fields(),
        },
    ];
    if workflow_mail_configured {
        specs.push(DslTypeSpec {
            r#type: "email".to_string(),
            fields: build_email_to_fields(),
        });
    }
    specs
}
