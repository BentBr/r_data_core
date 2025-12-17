use super::load_test_fixture;
use r_data_core_workflow::dsl::DslProgram;
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_chained_steps_two_steps_simple() {
    // Step 1: Normalize price field
    // Step 2: Read from Step 1 via PreviousStep, calculate total = price * 1.19
    let cfg = load_test_fixture("test_chained_two_steps.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    let input = json!({ "price": 100.0 });
    let results = prog.execute(&input).expect("execute");

    assert_eq!(results.len(), 2);
    assert_eq!(results[1].1["total"], json!(119.0));
}

#[tokio::test]
#[serial]
async fn test_chained_steps_three_steps_sequential() {
    // Step 1: Normalize price
    // Step 2: Calculate tax = price * 0.19
    // Step 3: Calculate total = price + tax
    let cfg = load_test_fixture("test_chained_three_steps.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    prog.validate().expect("valid dsl");

    let input = json!({ "price": 100.0 });
    let results = prog.execute(&input).expect("execute");

    assert_eq!(results.len(), 3);
    // Final result: 100 + (100 * 0.19) = 100 + 19 = 119
    assert_eq!(results[2].1["final_total"], json!(119.0));
}

#[tokio::test]
#[serial]
async fn test_step_zero_previous_step_validation_error() {
    // Step 0 cannot use PreviousStep source
    let cfg = load_test_fixture("test_step_zero_previous_step.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let result = prog.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("step 0") || err_msg.contains("first step"));
}

#[tokio::test]
#[serial]
async fn test_next_step_to_def_explicit() {
    // Test explicit NextStep ToDef variant
    let cfg = json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "api", "config": {} },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": { "price": "price" }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "next_step",
                    "mapping": { "base_price": "price" }
                }
            },
            {
                "from": {
                    "type": "previous_step",
                    "mapping": { "base_price": "base_price" }
                },
                "transform": {
                    "type": "arithmetic",
                    "target": "total",
                    "left": { "kind": "field", "field": "base_price" },
                    "op": "mul",
                    "right": { "kind": "const", "value": 1.19 }
                },
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
    prog.validate().expect("valid dsl");

    let input = json!({ "price": 100.0 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["total"], 119.0);
}

#[tokio::test]
#[serial]
async fn test_next_step_last_step_validation_error() {
    // Last step cannot use NextStep ToDef
    let cfg = json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "api", "config": {} },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": { "value": "value" }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "next_step",
                    "mapping": {}
                }
            }
        ]
    });

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let result = prog.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("last step") || err_msg.contains("no next step"));
}

#[tokio::test]
#[serial]
async fn test_next_step_with_mapping() {
    // NextStep with explicit field mapping
    let cfg = json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "api", "config": {} },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": { "a": "a", "b": "b" }
                },
                "transform": {
                    "type": "arithmetic",
                    "target": "sum",
                    "left": { "kind": "field", "field": "a" },
                    "op": "add",
                    "right": { "kind": "field", "field": "b" }
                },
                "to": {
                    "type": "next_step",
                    "mapping": { "total": "sum", "first": "a" }
                }
            },
            {
                "from": {
                    "type": "previous_step",
                    "mapping": { "total": "total", "first": "first" }
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
    prog.validate().expect("valid dsl");

    let input = json!({ "a": 5.0, "b": 10.0 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["total"], 15.0);
    assert_eq!(output["first"], 5.0);
    // 'b' should not be in output (not mapped in NextStep)
    assert!(output.get("b").is_none());
}

#[tokio::test]
#[serial]
async fn test_next_step_empty_mapping() {
    // NextStep with empty mapping passes through all fields
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
                    "type": "next_step",
                    "mapping": {}
                }
            },
            {
                "from": {
                    "type": "previous_step",
                    "mapping": {}
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
    prog.validate().expect("valid dsl");

    let input = json!({ "x": 10.0 });
    let output = prog.apply(&input).expect("apply");

    // Both x and y should be present (empty mapping passes through all)
    assert_eq!(output["x"], 10.0);
    assert_eq!(output["y"], 11.0);
}
