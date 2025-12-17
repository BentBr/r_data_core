use super::load_test_fixture;
use r_data_core_workflow::dsl::DslProgram;
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_arithmetic_string_to_number_invalid() {
    // Test strict type casting: string "abc" should fail
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "uri", "config": { "uri": "http://example.com/data.csv" } },
                "format": { "format_type": "csv", "options": {} },
                "mapping": { "value": "value" }
            },
            "transform": {
                "type": "arithmetic",
                "target": "result",
                "left": { "kind": "field", "field": "value" },
                "op": "add",
                "right": { "kind": "const", "value": 10.0 }
            },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            }
        }]
    });

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");

    let input = json!({ "value": "abc" });
    let result = prog.apply(&input);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cannot convert") || err_msg.contains("abc"));
}

#[tokio::test]
#[serial]
async fn test_casting_invalid_empty_string_to_number() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "value": "value" }
            },
            "transform": {
                "type": "arithmetic",
                "target": "result",
                "left": { "kind": "field", "field": "value" },
                "op": "add",
                "right": { "kind": "const", "value": 10.0 }
            },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            }
        }]
    });

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "value": "" });
    let result = prog.apply(&input);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cannot") || err_msg.contains("invalid"));
}

#[tokio::test]
#[serial]
async fn test_casting_invalid_alpha_string_to_number() {
    let cfg = load_test_fixture("test_casting_invalid_alpha.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "value": "hello" });
    let result = prog.apply(&input);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cannot convert") || err_msg.contains("invalid"));
}

#[tokio::test]
#[serial]
async fn test_casting_invalid_mixed_alphanumeric_to_number() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "value": "value" }
            },
            "transform": {
                "type": "arithmetic",
                "target": "result",
                "left": { "kind": "field", "field": "value" },
                "op": "add",
                "right": { "kind": "const", "value": 1.0 }
            },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            }
        }]
    });

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "value": "123abc" });
    let result = prog.apply(&input);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cannot convert") || err_msg.contains("invalid"));
}

#[tokio::test]
#[serial]
async fn test_casting_boolean_field_in_calculation() {
    // Boolean fields should fail conversion to number
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "active": "active" }
            },
            "transform": {
                "type": "arithmetic",
                "target": "result",
                "left": { "kind": "field", "field": "active" },
                "op": "add",
                "right": { "kind": "const", "value": 1.0 }
            },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            }
        }]
    });

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "active": true });
    let result = prog.apply(&input);
    assert!(result.is_err());
}
