#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Turning DSL failures into violations a caller can act on.
//!
//! Two kinds of failure reach a caller, and both used to arrive as prose:
//!
//! * **Parse failures** — a step serde cannot deserialize. `serde_path_to_error`
//!   gives the exact key, so the caller learns `steps[0].to.type` rather than
//!   "somewhere in step 0".
//! * **Semantic failures** — a step that parses but breaks a rule. The engine
//!   reports these as text beginning `DSL step N:`, so the step index is
//!   recoverable; anything else stays located at `steps`.
//!
//! Where serde names the legal alternatives, they are lifted out into
//! `legal_values` so a consumer can use them as data rather than parsing
//! English. That is what lets a form builder populate a dropdown, and an
//! assistant correct itself in one turn instead of guessing.

use serde_json::Value;

use crate::response::ValidationViolation;
use r_data_core_workflow::dsl::DslStep;

/// Deserialize the steps, collecting a located violation for each that fails.
///
/// # Errors
/// Returns one violation per unparseable step. A caller with three broken
/// steps should learn about all three, not just the first.
pub(super) fn parse_steps(raw_steps: &[Value]) -> Result<Vec<DslStep>, Vec<ValidationViolation>> {
    let mut steps = Vec::with_capacity(raw_steps.len());
    let mut violations = Vec::new();

    for (idx, raw) in raw_steps.iter().enumerate() {
        match serde_path_to_error::deserialize::<_, DslStep>(raw) {
            Ok(step) => steps.push(step),
            Err(e) => {
                let inner = e.inner().to_string();
                let path = e.path().to_string();
                // serde_path_to_error renders a failure at the root as ".".
                let json_path = if path.is_empty() || path == "." {
                    format!("steps[{idx}]")
                } else {
                    format!("steps[{idx}].{path}")
                };
                violations.push(ValidationViolation {
                    field: format!("steps[{idx}]"),
                    json_path: Some(json_path),
                    legal_values: legal_values_from(&inner),
                    message: inner,
                    code: Some("DSL_STEP_MALFORMED".to_string()),
                });
            }
        }
    }

    if violations.is_empty() {
        Ok(steps)
    } else {
        Err(violations)
    }
}

/// Locate a semantic validation failure as precisely as its message allows.
pub(super) fn semantic_violation(message: String) -> ValidationViolation {
    let json_path = step_index_from(&message)
        .map_or_else(|| "steps".to_string(), |idx| format!("steps[{idx}]"));
    ValidationViolation {
        field: "dsl".to_string(),
        json_path: Some(json_path),
        legal_values: Vec::new(),
        message,
        code: Some("DSL_INVALID".to_string()),
    }
}

/// Pull the step index out of an engine message such as
/// `"Validation error: DSL step 0: from.previous_step cannot be used ..."`.
///
/// Returns `None` when the message names no step — an empty program, say — so
/// the caller can stay honestly general rather than pointing at `steps[0]` and
/// sending someone to edit the wrong thing.
fn step_index_from(message: &str) -> Option<usize> {
    let after = message.split("DSL step ").nth(1)?;
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Pull the alternatives out of serde's `expected one of \`a\`, \`b\`` phrasing.
///
/// Returns an empty vec for any other failure shape; an empty list means "no
/// enumerable alternatives", never "none are legal".
fn legal_values_from(message: &str) -> Vec<String> {
    let Some(tail) = message.split("expected one of ").nth(1) else {
        return Vec::new();
    };
    // Backtick-quoted names alternate with the separators between them.
    tail.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locates_a_parse_failure_at_the_offending_key() {
        let steps = vec![serde_json::json!({
            "from": { "type": "trigger", "mapping": {} },
            "transform": { "type": "none" },
            "to": { "type": "entity_write", "mapping": {} }
        })];

        let violations = parse_steps(&steps).expect_err("should not parse");
        assert_eq!(violations[0].json_path.as_deref(), Some("steps[0].to.type"));
    }

    #[test]
    fn reports_every_broken_step_not_just_the_first() {
        let bad = serde_json::json!({ "from": { "type": "nope" } });
        let violations = parse_steps(&[bad.clone(), bad]).expect_err("should not parse");
        assert_eq!(violations.len(), 2, "both steps are broken: {violations:?}");
    }

    #[test]
    fn extracts_the_legal_alternatives() {
        let got = legal_values_from("unknown variant `x`, expected one of `format`, `entity`");
        assert_eq!(got, vec!["format".to_string(), "entity".to_string()]);
    }

    #[test]
    fn returns_no_alternatives_for_an_unrelated_message() {
        assert!(legal_values_from("invalid type: string, expected u32").is_empty());
    }

    #[test]
    fn finds_the_step_index_in_an_engine_message() {
        assert_eq!(
            step_index_from("Validation error: DSL step 3: something went wrong"),
            Some(3)
        );
    }

    #[test]
    fn finds_no_index_when_the_message_names_no_step() {
        assert_eq!(
            step_index_from("Validation error: DSL must contain at least one step"),
            None
        );
    }

    #[test]
    fn a_message_without_a_step_stays_located_at_steps() {
        let v = semantic_violation("DSL must contain at least one step".to_string());
        assert_eq!(v.json_path.as_deref(), Some("steps"));
    }
}
