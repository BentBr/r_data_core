use super::safe_field;
use crate::dsl::transform::validate_transform;
use crate::dsl::transform::{
    BuildPathTransform, GetOrCreateEntityTransform, ResolveEntityPathTransform, StringOperand,
    Transform,
};
use std::collections::HashMap;

fn resolve_entity_path(
    target_path: &str,
    target_uuid: Option<&str>,
    entity_type: &str,
    filters: HashMap<String, StringOperand>,
) -> Transform {
    Transform::ResolveEntityPath(ResolveEntityPathTransform {
        target_path: target_path.to_string(),
        target_uuid: target_uuid.map(ToString::to_string),
        entity_type: entity_type.to_string(),
        filters,
        value_transforms: None,
        fallback_path: None,
    })
}

fn safe_filters() -> HashMap<String, StringOperand> {
    HashMap::from([(
        "email".to_string(),
        StringOperand::Field {
            field: "input_email".to_string(),
        },
    )])
}

#[test]
fn valid_resolve_entity_path_ok() {
    let t = resolve_entity_path(
        "resolved_path",
        Some("resolved_uuid"),
        "user",
        safe_filters(),
    );
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn resolve_entity_path_unsafe_target_path_fails() {
    let t = resolve_entity_path("bad path!", None, "user", safe_filters());
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn resolve_entity_path_unsafe_target_uuid_fails() {
    let t = resolve_entity_path("resolved_path", Some("bad uuid!"), "user", safe_filters());
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn resolve_entity_path_none_target_uuid_ok() {
    let t = resolve_entity_path("resolved_path", None, "user", safe_filters());
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn resolve_entity_path_empty_entity_type_fails() {
    let t = resolve_entity_path("resolved_path", None, "", safe_filters());
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn resolve_entity_path_empty_filters_fails() {
    let t = resolve_entity_path("resolved_path", None, "user", HashMap::new());
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn resolve_entity_path_unsafe_filter_field_fails() {
    let filters = HashMap::from([(
        "email".to_string(),
        StringOperand::Field {
            field: "bad field!".to_string(),
        },
    )]);
    let t = resolve_entity_path("resolved_path", None, "user", filters);
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn resolve_entity_path_const_filter_value_ok() {
    let filters = HashMap::from([(
        "status".to_string(),
        StringOperand::ConstString {
            value: "active".to_string(),
        },
    )]);
    let t = resolve_entity_path("resolved_path", None, "user", filters);
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

fn build_path(target: &str, template: &str) -> Transform {
    Transform::BuildPath(BuildPathTransform {
        target: target.to_string(),
        template: template.to_string(),
        separator: None,
        field_transforms: None,
    })
}

#[test]
fn valid_build_path_ok() {
    let t = build_path("built_path", "/{category}/{sku}");
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn build_path_unsafe_target_fails() {
    let t = build_path("bad target!", "/{category}/{sku}");
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn build_path_empty_template_fails() {
    let t = build_path("built_path", "   ");
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

fn get_or_create(
    target_path: &str,
    target_uuid: Option<&str>,
    entity_type: &str,
    path_template: &str,
    create_field_data: Option<HashMap<String, StringOperand>>,
) -> Transform {
    Transform::GetOrCreateEntity(GetOrCreateEntityTransform {
        target_path: target_path.to_string(),
        target_uuid: target_uuid.map(ToString::to_string),
        entity_type: entity_type.to_string(),
        path_template: path_template.to_string(),
        create_field_data,
        path_separator: None,
    })
}

#[test]
fn valid_get_or_create_entity_ok() {
    let t = get_or_create(
        "path",
        Some("uuid"),
        "category",
        "/{name}",
        Some(HashMap::from([(
            "name".to_string(),
            StringOperand::Field {
                field: "input_name".to_string(),
            },
        )])),
    );
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}

#[test]
fn get_or_create_unsafe_target_path_fails() {
    let t = get_or_create("bad path!", None, "category", "/{name}", None);
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn get_or_create_unsafe_target_uuid_fails() {
    let t = get_or_create("path", Some("bad uuid!"), "category", "/{name}", None);
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn get_or_create_empty_entity_type_fails() {
    let t = get_or_create("path", None, "", "/{name}", None);
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn get_or_create_empty_path_template_fails() {
    let t = get_or_create("path", None, "category", "", None);
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn get_or_create_unsafe_create_field_data_fails() {
    let t = get_or_create(
        "path",
        None,
        "category",
        "/{name}",
        Some(HashMap::from([(
            "name".to_string(),
            StringOperand::Field {
                field: "bad field!".to_string(),
            },
        )])),
    );
    assert!(validate_transform(0, &t, &safe_field()).is_err());
}

#[test]
fn get_or_create_none_create_field_data_ok() {
    let t = get_or_create("path", None, "category", "/{name}", None);
    assert!(validate_transform(0, &t, &safe_field()).is_ok());
}
