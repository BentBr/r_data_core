use super::load_test_fixture;
use r_data_core_workflow::dsl::DslProgram;
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_fanout_one_input_two_calculations() {
    let cfg = load_test_fixture("test_fanout_one_input.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "price": 100.0 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["discounted_price"], 90.0);
}

#[tokio::test]
#[serial]
async fn test_fanout_multiple_transforms_from_same_source() {
    let cfg = load_test_fixture("test_fanout_multiple_transforms.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "a": 5.0, "b": 10.0 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["sum"], 15.0);
    assert_eq!(output["product"], 75.0); // (5 + 10) * 5
}

#[tokio::test]
#[serial]
async fn test_fanout_with_string_concatenation() {
    // Fan-out with mixed arithmetic and string operations
    let cfg = json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "api", "config": {} },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": { "first_name": "first_name", "last_name": "last_name", "age": "age" }
                },
                "transform": { "type": "none" },
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
                    "mapping": { "first_name": "first_name", "last_name": "last_name", "age": "age" }
                },
                "transform": {
                    "type": "concat",
                    "target": "full_name",
                    "left": { "kind": "field", "field": "first_name" },
                    "separator": " ",
                    "right": { "kind": "field", "field": "last_name" }
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
                    "mapping": { "age": "age", "full_name": "full_name" }
                },
                "transform": {
                    "type": "arithmetic",
                    "target": "age_next_year",
                    "left": { "kind": "field", "field": "age" },
                    "op": "add",
                    "right": { "kind": "const", "value": 1.0 }
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
    let input = json!({ "first_name": "John", "last_name": "Doe", "age": 30.0 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["full_name"], "John Doe");
    assert_eq!(output["age_next_year"], 31.0);
}

#[tokio::test]
#[serial]
async fn test_fanout_selective_field_propagation() {
    // Test that only explicitly mapped fields are available in next step
    let cfg = json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "api", "config": {} },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": { "a": "a", "b": "b", "c": "c" }
                },
                "transform": { "type": "none" },
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
                    "mapping": { "a": "a" }  // Only map 'a', not 'b' or 'c'
                },
                "transform": {
                    "type": "arithmetic",
                    "target": "result",
                    "left": { "kind": "field", "field": "a" },
                    "op": "mul",
                    "right": { "kind": "const", "value": 2.0 }
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
    let input = json!({ "a": 10.0, "b": 20.0, "c": 30.0 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["result"], 20.0);
    // 'b' and 'c' should not be in the final output because they weren't mapped in step 1
    assert!(output.get("b").is_none());
    assert!(output.get("c").is_none());
}

#[tokio::test]
#[serial]
async fn test_fanout_accumulated_fields() {
    // Each step adds new fields, accumulating data
    let cfg = json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "api", "config": {} },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": { "base": "base" }
                },
                "transform": { "type": "none" },
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
                    "mapping": { "base": "base" }
                },
                "transform": {
                    "type": "arithmetic",
                    "target": "doubled",
                    "left": { "kind": "field", "field": "base" },
                    "op": "mul",
                    "right": { "kind": "const", "value": 2.0 }
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
                    "mapping": { "base": "base", "doubled": "doubled" }
                },
                "transform": {
                    "type": "arithmetic",
                    "target": "tripled",
                    "left": { "kind": "field", "field": "base" },
                    "op": "mul",
                    "right": { "kind": "const", "value": 3.0 }
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
    let input = json!({ "base": 5.0 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["doubled"], 10.0);
    assert_eq!(output["tripled"], 15.0);
    // All accumulated fields should be present
}
