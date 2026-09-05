use super::safe_field;
use crate::dsl::to::{validate_to, EntityWriteMode, ToDef};
use std::collections::HashMap;

fn entity_def(
    entity_definition: &str,
    path: Option<&str>,
    mode: EntityWriteMode,
    mapping: HashMap<String, String>,
) -> ToDef {
    ToDef::Entity {
        entity_definition: entity_definition.to_string(),
        path: path.map(ToString::to_string),
        mode,
        identify: None,
        update_key: None,
        mapping,
    }
}

#[test]
fn entity_definition_empty_fails() {
    let def = entity_def("", None, EntityWriteMode::Create, HashMap::new());
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

#[test]
fn entity_path_none_is_ok_derived_at_runtime() {
    let def = entity_def("user", None, EntityWriteMode::Create, HashMap::new());
    assert!(validate_to(0, &def, &safe_field()).is_ok());
}

#[test]
fn entity_path_blank_string_fails() {
    let def = entity_def("user", Some("   "), EntityWriteMode::Update, HashMap::new());
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

#[test]
fn entity_path_present_ok() {
    let def = entity_def(
        "user",
        Some("/users"),
        EntityWriteMode::CreateOrUpdate,
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_ok());
}

#[test]
fn entity_all_write_modes_do_not_affect_validation() {
    for mode in [
        EntityWriteMode::Create,
        EntityWriteMode::Update,
        EntityWriteMode::CreateOrUpdate,
    ] {
        let def = entity_def("user", Some("/users"), mode, HashMap::new());
        assert!(validate_to(0, &def, &safe_field()).is_ok());
    }
}

#[test]
fn entity_unsafe_mapping_fails() {
    let def = entity_def(
        "user",
        None,
        EntityWriteMode::Create,
        HashMap::from([("bad field!".to_string(), "source".to_string())]),
    );
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

#[test]
fn next_step_empty_mapping_passes_through_all_fields() {
    let def = ToDef::NextStep {
        mapping: HashMap::new(),
    };
    assert!(validate_to(0, &def, &safe_field()).is_ok());
}

#[test]
fn next_step_unsafe_mapping_fails() {
    let def = ToDef::NextStep {
        mapping: HashMap::from([("bad field!".to_string(), "source".to_string())]),
    };
    assert!(validate_to(0, &def, &safe_field()).is_err());
}
