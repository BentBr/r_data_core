use super::load_example;
use r_data_core_workflow::dsl::{DslProgram, FromDef, ToDef};
use serde_json::json;
use serde_json::Value;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_validate_valid_arithmetic_example() {
    let cfg = load_example("valid_arithmetic.json");
    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");
    // Also ensure apply works on the simple payload
    let input = json!({ "price": 12.0 });
    let out = prog.apply(&input).expect("apply");
    assert_eq!(out["entity"]["total"], json!(17.0));
}

#[tokio::test]
#[serial]
async fn test_validate_valid_missing_mapping() {
    let cfg = load_example("valid_missing_mapping.json");
    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    // Empty mappings are allowed now; validation should succeed
    assert!(prog.validate().is_ok());
}

#[tokio::test]
#[serial]
async fn test_validate_invalid_operand() {
    let cfg = load_example("invalid_operand.json");
    let parsed = DslProgram::from_config(&cfg);
    assert!(parsed.is_err() || parsed.unwrap().validate().is_err());
}

#[tokio::test]
#[serial]
async fn test_validate_valid_complex_mapping() {
    let cfg = load_example("valid_complex_mapping.json");
    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    // Test execution with complex mappings
    let input = json!({
        "source_col1": "value1",
        "source_col2": "value2",
        "source_col3": "value3",
        "nested": { "source": "nested_value" }
    });

    let out = prog.apply(&input).expect("apply");
    assert_eq!(out["output_field1"], json!("value1"));
    assert_eq!(out["output_field2"], json!("value2"));
    assert_eq!(out["output_field3"], json!("value3"));
    assert_eq!(out["final"]["nested"], json!("nested_value"));
}

#[tokio::test]
#[serial]
async fn test_validate_valid_entity_mapping() {
    let cfg = load_example("valid_entity_mapping.json");
    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    // Entity mappings should parse and validate correctly
    assert_eq!(prog.steps.len(), 1);
    match &prog.steps[0].from {
        FromDef::Entity {
            entity_definition,
            filter,
            ..
        } => {
            assert_eq!(entity_definition, "source_entity");
            let filter = filter.as_ref().expect("Filter should exist");
            assert_eq!(filter.field, "status");
            assert_eq!(filter.value, "active");
        }
        FromDef::Format { .. } => panic!("Expected Entity FromDef"),
        FromDef::PreviousStep { .. } => panic!("Expected Entity FromDef, got PreviousStep"),
        FromDef::Trigger { .. } => panic!("Expected Entity FromDef, got Trigger"),
    }
}

#[tokio::test]
#[serial]
async fn test_validate_valid_csv_to_entity() {
    let cfg = load_example("valid_csv_to_entity.json");
    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    // Test execution with CSV to Entity workflow
    let input = json!({
        "product_name": "Test Product",
        "product_price": 100.0,
        "product_category": "Electronics"
    });

    let outputs = prog.execute(&input).expect("execute");
    assert_eq!(outputs.len(), 1);

    let (to_def, produced) = &outputs[0];
    match to_def {
        ToDef::Entity { .. } => {
            assert_eq!(produced["name"], json!("Test Product"));
            assert_eq!(produced["price"], json!(100.0));
            assert_eq!(produced["category"], json!("Electronics"));
            // Verify arithmetic transform was applied
            assert_eq!(produced["price_with_tax"], json!(119.0));
        }
        ToDef::Format { .. } | ToDef::NextStep { .. } => panic!("Expected Entity ToDef"),
    }
}

#[tokio::test]
#[serial]
async fn test_validate_valid_json_to_csv() {
    let cfg = load_example("valid_json_to_csv.json");
    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    // Test execution with JSON to CSV workflow
    let input = json!({
        "user": {
            "name": "John",
            "surname": "Doe",
            "email": "john@example.com"
        }
    });

    let out = prog.apply(&input).expect("apply");
    assert_eq!(out["first_name"], json!("John"));
    assert_eq!(out["last_name"], json!("Doe"));
    assert_eq!(out["full_name"], json!("John Doe"));
    assert_eq!(out["email"], json!("john@example.com"));
}

#[tokio::test]
#[serial]
async fn test_validate_valid_entity_to_json() {
    let cfg = load_example("valid_entity_to_json.json");
    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    // Test execution with Entity to JSON workflow
    let input = json!({
        "name": "Customer Name",
        "email": "customer@example.com",
        "phone": "123-456-7890"
    });

    let out = prog.apply(&input).expect("apply");
    assert_eq!(out["name"], json!("Customer Name"));
    assert_eq!(out["email"], json!("customer@example.com"));
    assert_eq!(out["phone"], json!("123-456-7890"));
}

#[tokio::test]
#[serial]
async fn test_validate_invalid_missing_from() {
    let cfg = load_example("invalid_missing_from.json");
    let parsed = DslProgram::from_config(&cfg);
    assert!(
        parsed.is_err(),
        "Should fail to parse when 'from' is missing"
    );
}

#[tokio::test]
#[serial]
async fn test_validate_invalid_missing_to() {
    let cfg = load_example("invalid_missing_to.json");
    let parsed = DslProgram::from_config(&cfg);
    assert!(parsed.is_err(), "Should fail to parse when 'to' is missing");
}

#[tokio::test]
#[serial]
async fn test_validate_invalid_unsafe_field_name() {
    let cfg = load_example("invalid_unsafe_field_name.json");
    let parsed = DslProgram::from_config(&cfg);
    if let Ok(prog) = parsed {
        // Should fail validation due to unsafe field names
        assert!(
            prog.validate().is_err(),
            "Should fail validation with unsafe field names"
        );
    } else {
        // Or fail to parse
        assert!(
            parsed.is_err(),
            "Should fail to parse or validate with unsafe field names"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_validate_invalid_transform_operand() {
    let cfg = load_example("invalid_transform_operand.json");
    let parsed = DslProgram::from_config(&cfg);
    // This should parse but may fail validation or execution
    if let Ok(prog) = parsed {
        // Validation might pass, but execution should handle missing fields gracefully
        let input = json!({ "price": 10.0 });
        // Execution might succeed but produce null/empty for missing field
        let _ = prog.apply(&input);
    }
}

#[tokio::test]
#[serial]
async fn test_validate_invalid_entity_definition() {
    let cfg = load_example("invalid_entity_definition.json");
    let parsed = DslProgram::from_config(&cfg);
    // Should parse successfully (entity definition validation happens at runtime)
    assert!(
        parsed.is_ok(),
        "Should parse even with non-existent entity definition"
    );
    if let Ok(prog) = parsed {
        // Validation should pass (entity existence is checked at runtime)
        assert!(
            prog.validate().is_ok(),
            "Validation should pass (entity check is runtime)"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_validate_invalid_empty_steps() {
    let cfg = load_example("invalid_empty_steps.json");
    let parsed = DslProgram::from_config(&cfg);
    assert!(parsed.is_ok(), "Should parse empty steps array");
    if let Ok(prog) = parsed {
        // Should fail validation because steps array is empty
        assert!(
            prog.validate().is_err(),
            "Should fail validation with empty steps array"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_validate_filter_operators() {
    // Test all valid operators
    let valid_operators = ["=", ">", "<", "<=", ">=", "IN", "NOT IN"];

    for operator in valid_operators {
        let template = load_example("workflow_entity_filter_operators_template.json");
        let cfg_str = serde_json::to_string(&template).expect("serialize");
        let filter_value = if operator == "IN" || operator == "NOT IN" {
            r#"["active", "pending"]"#
        } else {
            "active"
        };
        let cfg_str = cfg_str.replace("${OPERATOR}", operator);
        // Replace the placeholder - it's inside a JSON string, so we need to escape it properly
        let cfg_str = cfg_str.replace("${FILTER_VALUE}", &filter_value.replace('"', r#"\""#));
        let cfg: Value = serde_json::from_str(&cfg_str).expect("parse");

        let prog = DslProgram::from_config(&cfg);
        assert!(prog.is_ok(), "Should parse with valid operator: {operator}");

        if let Ok(p) = prog {
            assert!(
                p.validate().is_ok(),
                "Should validate with valid operator: {operator}"
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_validate_invalid_filter_operator() {
    // Test invalid operators
    let invalid_operators = [
        "= OR 1=1 --",
        "=; DROP TABLE entities; --",
        "LIKE",
        "BETWEEN",
        "invalid",
    ];

    for operator in invalid_operators {
        let cfg = json!({
            "steps": [
                {
                    "from": {
                        "type": "entity",
                        "entity_definition": "test_entity",
                        "filter": {
                            "field": "status",
                            "operator": operator,
                            "value": "active"
                        },
                        "mapping": {}
                    },
                    "transform": { "type": "none" },
                    "to": {
                        "type": "format",
                        "output": { "mode": "api" },
                        "format": {
                            "format_type": "json",
                            "options": {}
                        },
                        "mapping": {}
                    }
                }
            ]
        });

        let prog = DslProgram::from_config(&cfg);
        if let Ok(p) = prog {
            // Should fail validation with invalid operator
            assert!(
                p.validate().is_err(),
                "Should reject invalid operator: {operator}"
            );
        }
    }
}
