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
    async fn test_validate_invalid_missing_mapping() {
        let cfg = load_example("invalid_missing_mapping.json");
        let prog = DslProgram::from_config(&cfg).expect("parse dsl");
        let err = prog.validate().unwrap_err();
        assert!(err
            .to_string()
            .contains("mapping must contain at least one field"));
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_invalid_operand() {
        let cfg = load_example("invalid_operand.json");
        let parsed = DslProgram::from_config(&cfg);
        assert!(parsed.is_err() || parsed.unwrap().validate().is_err());
    }
}
