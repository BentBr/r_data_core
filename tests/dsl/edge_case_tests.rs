use super::load_test_fixture;
use r_data_core_workflow::dsl::DslProgram;
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_division_by_zero_error() {
    let cfg = load_test_fixture("test_division_by_zero.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");

    let input = json!({ "value": 100.0 });
    let result = prog.apply(&input);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("division by zero") || err_msg.contains("Division by zero"));
}

#[tokio::test]
#[serial]
async fn test_edge_case_null_field_in_arithmetic() {
    let cfg = load_test_fixture("test_edge_case_null_field.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "value": null });
    let result = prog.apply(&input);
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn test_edge_case_missing_field_in_arithmetic() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "other": "other" }
            },
            "transform": {
                "type": "arithmetic",
                "target": "result",
                "left": { "kind": "field", "field": "missing_field" },
                "op": "mul",
                "right": { "kind": "const", "value": 2.0 }
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
    let input = json!({ "other": 100 });
    let result = prog.apply(&input);
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn test_edge_case_null_field_in_concat() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "name": "name" }
            },
            "transform": {
                "type": "concat",
                "target": "greeting",
                "left": { "kind": "const_string", "value": "Hello, " },
                "separator": "",
                "right": { "kind": "field", "field": "name" }
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
    let input = json!({ "name": null });
    let result = prog.apply(&input);
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn test_edge_case_empty_json_input() {
    let cfg = load_test_fixture("test_edge_case_empty_input.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({});
    let output = prog.apply(&input).expect("apply");

    // Should produce empty output
    assert!(output.as_object().unwrap().is_empty() || output == json!({}));
}

#[tokio::test]
#[serial]
async fn test_edge_case_very_long_string_concat() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "text": "text" }
            },
            "transform": {
                "type": "concat",
                "target": "result",
                "left": { "kind": "field", "field": "text" },
                "separator": "",
                "right": { "kind": "field", "field": "text" }
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
    let long_string = "a".repeat(10000);
    let input = json!({ "text": long_string });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["result"].as_str().unwrap().len(), 20000);
}

#[tokio::test]
#[serial]
async fn test_edge_case_unicode_characters_in_concat() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "emoji": "emoji", "text": "text" }
            },
            "transform": {
                "type": "concat",
                "target": "result",
                "left": { "kind": "field", "field": "emoji" },
                "separator": " ",
                "right": { "kind": "field", "field": "text" }
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
    let input = json!({ "emoji": "🚀", "text": "こんにちは" });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["result"], "🚀 こんにちは");
}

#[tokio::test]
#[serial]
async fn test_edge_case_special_characters_in_field_values() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "special": "special" }
            },
            "transform": {
                "type": "concat",
                "target": "result",
                "left": { "kind": "const_string", "value": "Value: " },
                "separator": "",
                "right": { "kind": "field", "field": "special" }
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
    let input = json!({ "special": "a\"b'c\n\t\\d" });
    let output = prog.apply(&input).expect("apply");

    assert!(output["result"].as_str().unwrap().contains("Value:"));
}

#[tokio::test]
#[serial]
async fn test_edge_case_very_small_float_arithmetic() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "tiny": "tiny" }
            },
            "transform": {
                "type": "arithmetic",
                "target": "result",
                "left": { "kind": "field", "field": "tiny" },
                "op": "mul",
                "right": { "kind": "const", "value": 1_000_000.0 }
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
    let input = json!({ "tiny": 0.000_001 });
    let output = prog.apply(&input).expect("apply");

    assert!((output["result"].as_f64().unwrap() - 1.0).abs() < 0.0001);
}

#[tokio::test]
#[serial]
async fn test_edge_case_very_large_float_arithmetic() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "huge": "huge" }
            },
            "transform": {
                "type": "arithmetic",
                "target": "result",
                "left": { "kind": "field", "field": "huge" },
                "op": "div",
                "right": { "kind": "const", "value": 1_000_000.0 }
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
    let input = json!({ "huge": 1e15 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["result"], 1e9);
}

#[tokio::test]
#[serial]
async fn test_edge_case_chained_steps_with_empty_mappings() {
    // Test that steps can chain even with minimal mappings
    let cfg = json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "api", "config": {} },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": { "x": "x" }
                },
                "transform": {
                    "type": "arithmetic",
                    "target": "y",
                    "left": { "kind": "field", "field": "x" },
                    "op": "add",
                    "right": { "kind": "const", "value": 1.0 }
                },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": {}
                }
            },
            {
                "from": {
                    "type": "previous_step",
                    "mapping": {}  // Empty mapping
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": {}
                }
            }
        ]
    });

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "x": 10.0 });
    let output = prog.apply(&input).expect("apply");

    // First step should have produced y, but step 2 with empty mapping won't carry it forward
    // This tests that empty mappings are handled gracefully
    assert!(output.is_object());
}
