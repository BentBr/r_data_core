use super::load_example;
use r_data_core_workflow::dsl::{DslProgram, ToDef};
use serde_json::json;
use serde_json::Value;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_mapping_fix_active_to_published() {
    // Test the mapping fix: { "published": "active" } should map normalized "active" to destination "published"
    let cfg = load_example("mapping_fix_test.json");
    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    // Simulate CSV input row
    let input = json!({
        "email": "test@example.com",
        "active": true,
        "firstName": "John",
        "lastName": "Doe",
        "username": "jdoe"
    });

    // Execute the program
    let outputs = prog.execute(&input).expect("execute");
    assert_eq!(outputs.len(), 1);

    let (to_def, produced) = &outputs[0];
    match to_def {
        ToDef::Entity { .. } => {
            // Verify that "active" was mapped to "published"
            assert_eq!(produced["published"], json!(true));
            // Verify that "email" was mapped to both "email" and "entity_key"
            assert_eq!(produced["email"], json!("test@example.com"));
            assert_eq!(produced["entity_key"], json!("test@example.com"));
            // Verify that "active" is NOT in the output (should be "published" instead)
            assert!(!produced.as_object().unwrap().contains_key("active"));
            // Verify other fields are correctly mapped
            assert_eq!(produced["firstName"], json!("John"));
            assert_eq!(produced["lastName"], json!("Doe"));
            assert_eq!(produced["username"], json!("jdoe"));
        }
        ToDef::Format { .. } | ToDef::NextStep { .. } => panic!("Expected Entity ToDef"),
    }
}

#[tokio::test]
#[serial]
async fn test_mapping_fix_with_apply() {
    // Test that apply() method also works correctly with the mapping fix
    let cfg = load_example("mapping_fix_test.json");
    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    let input = json!({
        "email": "test@example.com",
        "active": false,
        "firstName": "Jane",
        "lastName": "Smith",
        "username": "jsmith"
    });

    // Test apply() method
    let out = prog.apply(&input).expect("apply");

    // Verify mapping fix works in apply()
    assert_eq!(out["published"], json!(false));
    assert_eq!(out["email"], json!("test@example.com"));
    assert_eq!(out["entity_key"], json!("test@example.com"));
    assert!(!out.as_object().unwrap().contains_key("active"));
}

#[tokio::test]
#[serial]
async fn test_mapping_fallback_with_empty_mapping() {
    // Test that empty mapping passes through all fields
    let cfg = load_example("workflow_export_entity_empty_mapping.json");
    // Replace entity_type placeholder
    let cfg_str = serde_json::to_string(&cfg).expect("serialize");
    let cfg_str = cfg_str.replace("${ENTITY_TYPE}", "test_entity");
    let cfg: Value = serde_json::from_str(&cfg_str).expect("parse");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    // Test with input that has multiple fields
    let input = json!({
        "name": "Test",
        "email": "test@example.com",
        "age": 30,
        "status": "active"
    });

    let outputs = prog.execute(&input).expect("execute");
    assert_eq!(outputs.len(), 1);

    let (_, produced) = &outputs[0];

    // All fields should be present (empty mapping = pass through)
    assert!(produced["name"].is_string(), "Should include name field");
    assert!(produced["email"].is_string(), "Should include email field");
    assert!(produced["age"].is_number(), "Should include age field");
    assert!(
        produced["status"].is_string(),
        "Should include status field"
    );
}
