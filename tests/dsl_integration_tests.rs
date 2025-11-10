#[cfg(test)]
mod tests {
    use r_data_core::workflow::dsl::DslProgram;
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
            r_data_core::workflow::dsl::ToDef::Entity { .. } => {
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
            _ => panic!("Expected Entity ToDef"),
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
}
