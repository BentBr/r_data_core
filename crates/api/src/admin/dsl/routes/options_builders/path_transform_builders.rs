#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Option specs for the entity-path transforms.
//!
//! These three run in the services layer rather than the synchronous DSL
//! executor, which is why they were missing from the catalogue: nothing in the
//! `from`/`transform`/`to` option surface pointed at them. Grouped here rather
//! than in `transform_builders.rs` to keep that file under the length cap.

use crate::admin::dsl::models::DslFieldSpec;

/// Build field specifications for the `resolve_entity_path` transform type
///
/// Looks an entity up by filters and writes its path (and optionally its UUID)
/// into the normalized data.
pub(super) fn build_resolve_entity_path_transform_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "target_path".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "target_uuid".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "entity_type".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "filters".into(),
            r#type: "object".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "value_transforms".into(),
            r#type: "map<string,string>".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "fallback_path".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
    ]
}

/// Build field specifications for the `build_path` transform type
///
/// Builds a path from a template with `{field}` placeholders.
pub(super) fn build_build_path_transform_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "target".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "template".into(),
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
            name: "field_transforms".into(),
            r#type: "map<string,string>".into(),
            required: false,
            options: None,
        },
    ]
}

/// Build field specifications for the `get_or_create_entity` transform type
///
/// Resolves an entity by path, creating it when absent.
pub(super) fn build_get_or_create_entity_transform_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "target_path".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "target_uuid".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "entity_type".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "path_template".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "create_field_data".into(),
            r#type: "object".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "path_separator".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
    ]
}
