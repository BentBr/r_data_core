#![allow(clippy::unwrap_used)]

use super::DslProgram;
use serde_json::json;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Minimal single-step config using format→format with a none transform.
fn single_step_config() -> serde_json::Value {
    json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "uri", "config": { "uri": "http://example.com/data.json" }, "auth": null },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            },
            "transform": { "type": "none" },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            }
        }]
    })
}

// ── DslProgram::from_config ───────────────────────────────────────────────────

#[test]
fn from_config_missing_steps_key_fails() {
    let result = DslProgram::from_config(&json!({}));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("steps"), "expected 'steps' in: {msg}");
}

#[test]
fn from_config_steps_not_array_fails() {
    let result = DslProgram::from_config(&json!({ "steps": "not_an_array" }));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("array"), "expected 'array' in: {msg}");
}

#[test]
fn from_config_invalid_step_json_fails() {
    let result = DslProgram::from_config(&json!({ "steps": [{"type": "bogus"}] }));
    assert!(result.is_err());
}

#[test]
fn from_config_valid_single_step() {
    let prog = DslProgram::from_config(&single_step_config()).unwrap();
    assert_eq!(prog.steps.len(), 1);
    assert!(prog.on_complete.is_none());
}

#[test]
fn from_config_ignores_invalid_on_complete_silently() {
    let mut cfg = single_step_config();
    cfg["on_complete"] = json!("not_an_object");
    let prog = DslProgram::from_config(&cfg).unwrap();
    // Invalid on_complete is silently ignored (serde_json::from_value returns None via .ok())
    assert!(prog.on_complete.is_none());
}

// ── DslProgram::validate ──────────────────────────────────────────────────────

#[test]
fn validate_empty_steps_fails() {
    let prog = DslProgram {
        steps: vec![],
        on_complete: None,
    };
    let err = prog.validate().unwrap_err().to_string();
    assert!(err.contains("at least one step"), "got: {err}");
}

#[test]
fn validate_next_step_as_last_step_fails() {
    let config = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "uri", "config": { "uri": "http://example.com/d.json" }, "auth": null },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            },
            "transform": { "type": "none" },
            "to": { "type": "next_step", "mapping": {} }
        }]
    });
    let prog = DslProgram::from_config(&config).unwrap();
    let err = prog.validate().unwrap_err().to_string();
    assert!(
        err.contains("NextStep") || err.contains("next step"),
        "got: {err}"
    );
}

#[test]
fn validate_valid_program_passes() {
    let prog = DslProgram::from_config(&single_step_config()).unwrap();
    assert!(prog.validate().is_ok());
}

// ── DslProgram::prepare_step ──────────────────────────────────────────────────

#[test]
fn prepare_step_out_of_bounds_fails() {
    let prog = DslProgram::from_config(&single_step_config()).unwrap();
    let result = prog.prepare_step(99, &json!({}), None);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("out of bounds"), "got: {msg}");
}

#[test]
fn prepare_step_previous_step_at_index_zero_fails() {
    let config = json!({
        "steps": [{
            "from": { "type": "previous_step", "mapping": {} },
            "transform": { "type": "none" },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            }
        }]
    });
    let prog = DslProgram::from_config(&config).unwrap();
    let result = prog.prepare_step(0, &json!({}), None);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Step 0") || msg.contains("PreviousStep"),
        "got: {msg}"
    );
}

#[test]
fn prepare_step_previous_step_missing_previous_output_fails() {
    let config = json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "uri", "config": { "uri": "http://example.com/d.json" }, "auth": null },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": { "type": "next_step", "mapping": {} }
            },
            {
                "from": { "type": "previous_step", "mapping": {} },
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
    let prog = DslProgram::from_config(&config).unwrap();
    // previous_step_output is None → should error
    let result = prog.prepare_step(1, &json!({}), None);
    assert!(result.is_err());
}

#[test]
fn prepare_step_format_source_normalizes_input() {
    let prog = DslProgram::from_config(&single_step_config()).unwrap();
    let input = json!({ "foo": "bar" });
    let (normalized, _transform) = prog.prepare_step(0, &input, None).unwrap();
    assert_eq!(normalized["foo"], json!("bar"));
}

// ── DslProgram::finalize_step ─────────────────────────────────────────────────

#[test]
fn finalize_step_out_of_bounds_fails() {
    let prog = DslProgram::from_config(&single_step_config()).unwrap();
    let result = prog.finalize_step(99, &json!({}));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("out of bounds"), "got: {msg}");
}

#[test]
fn finalize_step_no_mapping_passes_through() {
    let prog = DslProgram::from_config(&single_step_config()).unwrap();
    let normalized = json!({ "x": 1 });
    let (_to_def, produced) = prog.finalize_step(0, &normalized).unwrap();
    assert_eq!(produced["x"], json!(1));
}

#[test]
fn finalize_step_with_mapping_projects_fields() {
    let config = json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "uri", "config": { "uri": "http://example.com/d.json" }, "auth": null },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "val": "val" }
            },
            "transform": { "type": "none" },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "out_val": "val" }
            }
        }]
    });
    let prog = DslProgram::from_config(&config).unwrap();
    let normalized = json!({ "val": 42 });
    let (_to_def, produced) = prog.finalize_step(0, &normalized).unwrap();
    assert_eq!(produced["out_val"], json!(42));
}

// ── DslProgram::get_next_step_input ──────────────────────────────────────────

#[test]
fn get_next_step_input_out_of_bounds_fails() {
    let prog = DslProgram::from_config(&single_step_config()).unwrap();
    let result = prog.get_next_step_input(99, &json!({}), &json!({}));
    assert!(result.is_err());
}

#[test]
fn get_next_step_input_format_to_returns_normalized() {
    let prog = DslProgram::from_config(&single_step_config()).unwrap();
    let normalized = json!({ "norm": true });
    let produced = json!({ "prod": true });
    let out = prog.get_next_step_input(0, &normalized, &produced).unwrap();
    // Format ToDef → returns normalized
    assert_eq!(out["norm"], json!(true));
    assert!(out.get("prod").is_none());
}

#[test]
fn get_next_step_input_next_step_to_returns_produced() {
    let config = json!({
        "steps": [
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "uri", "config": { "uri": "http://example.com/d.json" }, "auth": null },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": { "type": "next_step", "mapping": {} }
            },
            {
                "from": {
                    "type": "format",
                    "source": { "source_type": "uri", "config": { "uri": "http://example.com/d.json" }, "auth": null },
                    "format": { "format_type": "json", "options": {} },
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
    let prog = DslProgram::from_config(&config).unwrap();
    let normalized = json!({ "norm": true });
    let produced = json!({ "prod": true });
    // Step 0 has NextStep ToDef → should return produced
    let out = prog.get_next_step_input(0, &normalized, &produced).unwrap();
    assert_eq!(out["prod"], json!(true));
    assert!(out.get("norm").is_none());
}

// ── DslProgram::apply_build_path ──────────────────────────────────────────────

#[test]
fn apply_build_path_non_build_path_transform_is_noop() {
    use crate::dsl::transform::Transform;
    let mut normalized = json!({ "x": 1 });
    let result = DslProgram::apply_build_path(0, &Transform::None, &mut normalized);
    assert!(result.is_ok());
    // normalized unchanged
    assert_eq!(normalized["x"], json!(1));
}

// ── BuildPath defer semantics (apply_sync_transform `defer_build_path`) ─────────
//
// BuildPath can depend on the output of async transforms (resolved later in the
// services layer), so step-by-step execution MUST defer it: `prepare_step` does
// not build the path, while full `execute`/`apply` build it inline. These tests
// pin both branches and that they converge.

/// Single step whose `BuildPath` template references fields present in the input.
fn build_path_config() -> serde_json::Value {
    json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": { "source_type": "uri", "config": { "uri": "http://example.com/data.json" }, "auth": null },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            },
            "transform": {
                "type": "build_path",
                "target": "result_path",
                "template": "{base}/{name}",
                "separator": "/"
            },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": {}
            }
        }]
    })
}

#[test]
fn execute_applies_build_path_inline() {
    let prog = DslProgram::from_config(&build_path_config()).unwrap();
    let input = json!({ "base": "alpha", "name": "beta" });
    let results = prog.execute(&input).unwrap();
    let (_to, produced) = &results[0];
    assert!(
        produced
            .get("result_path")
            .and_then(|v| v.as_str())
            .is_some(),
        "full execution must build the path inline: {produced}"
    );
}

#[test]
fn prepare_step_defers_build_path_until_apply() {
    use crate::dsl::transform::Transform;
    let prog = DslProgram::from_config(&build_path_config()).unwrap();
    let input = json!({ "base": "alpha", "name": "beta" });

    // Step-by-step prepare must NOT build the path (it may depend on async results).
    let (mut normalized, transform) = prog.prepare_step(0, &input, None).unwrap();
    assert!(
        normalized.get("result_path").is_none(),
        "prepare_step must defer BuildPath, not apply it: {normalized}"
    );
    assert!(matches!(transform, Transform::BuildPath(_)));

    // Applying it explicitly afterwards builds the path.
    DslProgram::apply_build_path(0, transform, &mut normalized).unwrap();
    assert!(
        normalized
            .get("result_path")
            .and_then(|v| v.as_str())
            .is_some(),
        "apply_build_path must build the deferred path: {normalized}"
    );

    // The deferred result matches what full execution produces inline.
    let produced = &prog.execute(&input).unwrap()[0].1;
    assert_eq!(produced["result_path"], normalized["result_path"]);
}
