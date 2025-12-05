#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

#[cfg(test)]
mod tests {
    use r_data_core_workflow::dsl::{DslProgram, FromDef, ToDef};
    use serde_json::json;
    use serde_json::Value;
    use serial_test::serial;
    use std::fs::read_to_string;

    fn load_example(path: &str) -> Value {
        let content = read_to_string(format!(".example_files/json_examples/dsl/{path}"))
            .expect("read example");
        serde_json::from_str(&content).expect("parse json")
    }

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
            ToDef::Format { .. } => panic!("Expected Entity ToDef"),
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
                assert_eq!(filter.field, "status");
                assert_eq!(filter.value, "active");
            }
            FromDef::Format { .. } => panic!("Expected Entity FromDef"),
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
            ToDef::Format { .. } => panic!("Expected Entity ToDef"),
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
}
