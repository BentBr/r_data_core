#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::admin::dsl::models::{DslFieldSpec, DslTypeSpec};

/// Build field specifications for format FROM type
pub(super) fn build_format_from_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "source.source_type".into(),
            r#type: "string".into(),
            required: true,
            options: Some(vec!["uri".into(), "api".into(), "file".into()]),
        },
        DslFieldSpec {
            name: "source.config.uri".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "source.config.endpoint".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "source.auth".into(),
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

/// Build field specifications for entity FROM type
pub(super) fn build_entity_from_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "entity_definition".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "filter".into(),
            r#type: "object".into(),
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

/// Build field specifications for `previous_step` FROM type
///
/// Reads the previous step's normalized data. Invalid in step 0.
pub(super) fn build_previous_step_from_fields() -> Vec<DslFieldSpec> {
    vec![DslFieldSpec {
        name: "mapping".into(),
        r#type: "map<string,string>".into(),
        required: true,
        options: None,
    }]
}

/// Build field specifications for `trigger` FROM type
///
/// Accepts a GET at `/api/v1/workflows/{uuid}/trigger` with no payload, so the
/// mapping is typically empty. Only valid in step 0.
pub(super) fn build_trigger_from_fields() -> Vec<DslFieldSpec> {
    vec![DslFieldSpec {
        name: "mapping".into(),
        r#type: "map<string,string>".into(),
        required: true,
        options: None,
    }]
}

/// Build FROM type specifications
#[must_use]
pub fn build_from_type_specs() -> Vec<DslTypeSpec> {
    vec![
        DslTypeSpec {
            r#type: "format".to_string(),
            fields: build_format_from_fields(),
        },
        DslTypeSpec {
            r#type: "entity".to_string(),
            fields: build_entity_from_fields(),
        },
        DslTypeSpec {
            r#type: "previous_step".to_string(),
            fields: build_previous_step_from_fields(),
        },
        DslTypeSpec {
            r#type: "trigger".to_string(),
            fields: build_trigger_from_fields(),
        },
    ]
}
