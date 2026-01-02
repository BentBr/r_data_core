#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_workflow::dsl::DslProgram;
use serde_json::json;
use serde_json::Value;
use std::fs::read_to_string;

/// Load an example JSON file from `.example_files/json_examples/dsl/`
fn load_example(path: &str) -> Value {
    let content =
        read_to_string(format!(".example_files/json_examples/dsl/{path}")).expect("read example");
    serde_json::from_str(&content).expect("parse json")
}

#[test]
fn test_build_path_transform_execution() {
    let config = load_example("workflow_build_path_transform.json");

    let prog = DslProgram::from_config(&config).unwrap();
    prog.validate().unwrap();

    let input = json!({
        "license_key_id": "ABC-123"
    });

    let result = prog.apply(&input).unwrap();
    assert_eq!(result["path"], json!("/statistics_instance/ABC-123"));
}

#[test]
fn test_build_path_transform_with_field_transforms() {
    let config = load_example("workflow_build_path_transform_with_field_transforms.json");

    let prog = DslProgram::from_config(&config).unwrap();
    prog.validate().unwrap();

    let input = json!({
        "license_key_id": "ABC-123",
        "instance_name": "Production Instance"
    });

    let result = prog.apply(&input).unwrap();
    assert_eq!(
        result["path"],
        json!("/statistics_instance/abc-123/production-instance")
    );
}

#[test]
fn test_build_path_transform_missing_field() {
    let config = load_example("workflow_build_path_transform.json");

    let prog = DslProgram::from_config(&config).unwrap();
    prog.validate().unwrap();

    let input = json!({
        "other_field": "value"
    });

    // Should fail when field is missing
    let result = prog.apply(&input);
    assert!(result.is_err());
}

#[test]
fn test_resolve_entity_path_transform_validation() {
    // Test that ResolveEntityPath transform is validated but not executed in DSL layer
    let config = load_example("workflow_resolve_entity_path_transform.json");

    let prog = DslProgram::from_config(&config).unwrap();
    // Validation should pass (async execution happens in services layer)
    assert!(prog.validate().is_ok());
}

#[test]
fn test_get_or_create_entity_transform_validation() {
    // Test that GetOrCreateEntity transform is validated but not executed in DSL layer
    let config = load_example("workflow_get_or_create_entity_transform.json");

    let prog = DslProgram::from_config(&config).unwrap();
    // Validation should pass (async execution happens in services layer)
    assert!(prog.validate().is_ok());
}
