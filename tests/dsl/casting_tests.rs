use super::load_test_fixture;
use r_data_core_workflow::dsl::DslProgram;
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_arithmetic_string_to_number_valid() {
    // Test strict type casting: string "123.45" should be cast to number
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
                "mapping": { "result": "result" }
            }
        }]
    });

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");

    let input = json!({ "value": "123.45" });
    let result = prog.apply(&input).expect("should succeed");
    assert_eq!(result["result"], json!(133.45)); // 123.45 + 10
}

#[tokio::test]
#[serial]
async fn test_casting_integer_string_to_number() {
    let cfg = load_test_fixture("test_casting_integer_string.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "price": "42" });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["doubled"], 84.0);
}

#[tokio::test]
#[serial]
async fn test_casting_float_string_to_number() {
    let cfg = load_test_fixture("test_casting_float_string.json");

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");
    let input = json!({ "value": "123.45" });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["result"], 133.95);
}

#[tokio::test]
#[serial]
async fn test_casting_negative_string_to_number() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "temp": "temp" }
            },
            "transform": {
                "type": "arithmetic",
                "target": "celsius",
                "left": { "kind": "field", "field": "temp" },
                "op": "sub",
                "right": { "kind": "const", "value": 32.0 }
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
    let input = json!({ "temp": "-10.5" });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["celsius"], -42.5);
}

#[tokio::test]
#[serial]
async fn test_casting_scientific_notation_string_to_number() {
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
    let input = json!({ "value": "1.5e2" }); // 150.0
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["result"], 300.0);
}

#[tokio::test]
#[serial]
async fn test_casting_whitespace_string_to_number() {
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
                "right": { "kind": "const", "value": 5.0 }
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
    let input = json!({ "value": "  42  " });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["result"], 47.0);
}

#[tokio::test]
#[serial]
async fn test_casting_zero_string_to_number() {
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
                "right": { "kind": "const", "value": 100.0 }
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
    let input = json!({ "value": "0" });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["result"], 100.0);
}

#[tokio::test]
#[serial]
async fn test_concat_number_to_string_smart_formatting() {
    // Test smart formatting: 123.0 → "123" (not "123.0")
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "uri", "config": { "uri": "http://example.com/data.csv" } },
                "format": { "format_type": "csv", "options": {} },
                "mapping": { "value": "value" }
            },
            "transform": {
                "type": "concat",
                "target": "result",
                "left": { "kind": "field", "field": "value" },
                "separator": "_",
                "right": { "kind": "const_string", "value": "suffix" }
            },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "result": "result" }
            }
        }]
    });

    let prog = DslProgram::from_config(&cfg).expect("parse dsl");

    // Test integer-valued float
    let input = json!({ "value": 123.0 });
    let result = prog.apply(&input).expect("should succeed");
    assert_eq!(result["result"], json!("123_suffix")); // Not "123.0_suffix"

    // Test float with decimals
    let input2 = json!({ "value": 123.45 });
    let result2 = prog.apply(&input2).expect("should succeed");
    assert_eq!(result2["result"], json!("123.45_suffix"));
}

#[tokio::test]
#[serial]
async fn test_casting_integer_to_string_concat() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "count": "count" }
            },
            "transform": {
                "type": "concat",
                "target": "message",
                "left": { "kind": "const_string", "value": "Count: " },
                "separator": "",
                "right": { "kind": "field", "field": "count" }
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
    let input = json!({ "count": 42 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["message"], "Count: 42");
}

#[tokio::test]
#[serial]
async fn test_casting_float_to_string_preserves_decimals() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "price": "price" }
            },
            "transform": {
                "type": "concat",
                "target": "label",
                "left": { "kind": "const_string", "value": "$" },
                "separator": "",
                "right": { "kind": "field", "field": "price" }
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
    let input = json!({ "price": 19.99 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["label"], "$19.99");
}

#[tokio::test]
#[serial]
async fn test_casting_zero_to_string() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "value": "value" }
            },
            "transform": {
                "type": "concat",
                "target": "result",
                "left": { "kind": "const_string", "value": "Value=" },
                "separator": "",
                "right": { "kind": "field", "field": "value" }
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
    let input = json!({ "value": 0 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["result"], "Value=0");
}

#[tokio::test]
#[serial]
async fn test_casting_negative_number_to_string() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "temp": "temp" }
            },
            "transform": {
                "type": "concat",
                "target": "display",
                "left": { "kind": "field", "field": "temp" },
                "separator": "",
                "right": { "kind": "const_string", "value": "°C" }
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
    let input = json!({ "temp": -15.5 });
    let output = prog.apply(&input).expect("apply");

    assert_eq!(output["display"], "-15.5°C");
}

#[tokio::test]
#[serial]
async fn test_casting_large_number_to_string() {
    let cfg = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "api", "config": {} },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "population": "population" }
            },
            "transform": {
                "type": "concat",
                "target": "report",
                "left": { "kind": "const_string", "value": "Population: " },
                "separator": "",
                "right": { "kind": "field", "field": "population" }
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
    let input = json!({ "population": 7_800_000_000.0 });
    let output = prog.apply(&input).expect("apply");

    assert!(output["report"].as_str().unwrap().contains("7800000000"));
}
