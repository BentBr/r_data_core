#![allow(clippy::unwrap_used)]

//! Contract tests for the DSL option specs the admin UI renders its step forms
//! from.
//!
//! These specs are hand-maintained lists that have to stay in step with what
//! the workflow engine actually accepts. The tests check the invariants a
//! consumer relies on — every field declares a published type, every option
//! list is usable, names do not collide — plus the mail gating, rather than
//! restating each literal.

use super::{build_from_type_specs, build_to_type_specs, build_transform_type_specs};
use crate::admin::dsl::models::{DslFieldSpec, DslTypeSpec};
use std::collections::HashSet;

/// The complete field-type vocabulary these hand-maintained specs use.
///
/// Pinned deliberately: the list is consumed as documentation and by anything
/// generating a form from it, so a new spelling should be a conscious addition
/// rather than something that slips in. Note `map<string,string>` and
/// `object<string, string>` are two spellings of the same shape — kept as-is
/// because they are already published on the API.
const KNOWN_FIELD_TYPES: &[&str] = &[
    "array<string>",
    "boolean",
    "enum",
    "map<string,string>",
    "number",
    "object",
    "object<string, string>",
    "string",
    "string(1)",
];

fn all_specs() -> Vec<(&'static str, Vec<DslTypeSpec>)> {
    vec![
        ("from", build_from_type_specs()),
        ("to", build_to_type_specs(true)),
        ("transform", build_transform_type_specs(true)),
    ]
}

fn field_names(spec: &DslTypeSpec) -> Vec<&str> {
    spec.fields.iter().map(|f| f.name.as_str()).collect()
}

fn find<'a>(specs: &'a [DslTypeSpec], name: &str) -> &'a DslTypeSpec {
    specs
        .iter()
        .find(|s| s.r#type == name)
        .unwrap_or_else(|| panic!("no `{name}` spec"))
}

fn field<'a>(spec: &'a DslTypeSpec, name: &str) -> &'a DslFieldSpec {
    spec.fields
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no `{name}` field on `{}`", spec.r#type))
}

#[test]
fn every_field_uses_a_published_type() {
    for (group, specs) in all_specs() {
        for spec in &specs {
            for f in &spec.fields {
                assert!(
                    KNOWN_FIELD_TYPES.contains(&f.r#type.as_str()),
                    "{group}/{}/{} declares type `{}`, which is not in the \
                     published vocabulary — add it to KNOWN_FIELD_TYPES if intended",
                    spec.r#type,
                    f.name,
                    f.r#type
                );
            }
        }
    }
}

/// An `enum` without options is unusable. A non-enum may still carry options —
/// `source.source_type` is declared `string` with a suggested set — so the
/// invariant is that any list present is usable, not that only enums have one.
#[test]
fn every_option_list_is_usable() {
    for (group, specs) in all_specs() {
        for spec in &specs {
            for f in &spec.fields {
                if f.r#type == "enum" {
                    assert!(
                        f.options.is_some(),
                        "{group}/{}/{} is an enum with no options",
                        spec.r#type,
                        f.name
                    );
                }
                if let Some(options) = &f.options {
                    assert!(
                        !options.is_empty(),
                        "{group}/{}/{} has an empty option list",
                        spec.r#type,
                        f.name
                    );
                    assert!(
                        options.iter().all(|o| !o.trim().is_empty()),
                        "{group}/{}/{} has a blank option",
                        spec.r#type,
                        f.name
                    );
                }
            }
        }
    }
}

#[test]
fn enum_options_are_unique() {
    for (group, specs) in all_specs() {
        for spec in &specs {
            for f in &spec.fields {
                let Some(options) = &f.options else { continue };
                let unique: HashSet<&String> = options.iter().collect();
                assert_eq!(
                    unique.len(),
                    options.len(),
                    "{group}/{}/{} repeats an option",
                    spec.r#type,
                    f.name
                );
            }
        }
    }
}

#[test]
fn field_names_are_unique_within_a_type() {
    for (group, specs) in all_specs() {
        for spec in &specs {
            let names = field_names(spec);
            let unique: HashSet<&&str> = names.iter().collect();
            assert_eq!(
                unique.len(),
                names.len(),
                "{group}/{} repeats a field name",
                spec.r#type
            );
        }
    }
}

#[test]
fn no_field_name_is_blank_or_padded() {
    for (group, specs) in all_specs() {
        for spec in &specs {
            assert!(!spec.r#type.is_empty(), "{group} has a spec with no type");
            for f in &spec.fields {
                assert!(
                    !f.name.is_empty(),
                    "{group}/{} has a blank field",
                    spec.r#type
                );
                assert_eq!(
                    f.name.trim(),
                    f.name,
                    "{group}/{}/{} is padded",
                    spec.r#type,
                    f.name
                );
            }
        }
    }
}

#[test]
fn type_names_are_unique_within_a_group() {
    for (group, specs) in all_specs() {
        let names: Vec<&str> = specs.iter().map(|s| s.r#type.as_str()).collect();
        let unique: HashSet<&&str> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "{group} repeats a type name");
    }
}

// --- mail gating ----------------------------------------------------------
// The email step is only offered when a workflow mailer is configured;
// otherwise the UI would render a form for something the engine cannot run.

#[test]
fn email_steps_appear_only_when_mail_is_configured() {
    let to_without = build_to_type_specs(false);
    assert!(!to_without.iter().any(|s| s.r#type == "email"));
    assert!(build_to_type_specs(true)
        .iter()
        .any(|s| s.r#type == "email"));

    let transform_without = build_transform_type_specs(false);
    assert!(!transform_without.iter().any(|s| s.r#type == "send_email"));
    assert!(build_transform_type_specs(true)
        .iter()
        .any(|s| s.r#type == "send_email"));
}

#[test]
fn mail_gating_changes_nothing_else() {
    let strip_mail = |specs: Vec<DslTypeSpec>, gated: &str| -> Vec<String> {
        specs
            .into_iter()
            .filter(|s| s.r#type != gated)
            .map(|s| s.r#type)
            .collect()
    };

    assert_eq!(
        strip_mail(build_to_type_specs(true), "email"),
        strip_mail(build_to_type_specs(false), "email"),
    );
    assert_eq!(
        strip_mail(build_transform_type_specs(true), "send_email"),
        strip_mail(build_transform_type_specs(false), "send_email"),
    );
}

// --- the step types the engine actually accepts ---------------------------

#[test]
fn from_offers_the_engine_step_types() {
    let specs = build_from_type_specs();
    let names: Vec<&str> = specs.iter().map(|s| s.r#type.as_str()).collect();
    assert!(names.contains(&"format"), "got {names:?}");
    assert!(names.contains(&"entity"), "got {names:?}");
}

#[test]
fn to_offers_the_engine_step_types() {
    let names: Vec<String> = build_to_type_specs(true)
        .into_iter()
        .map(|s| s.r#type)
        .collect();
    for expected in ["format", "entity", "email"] {
        assert!(
            names.iter().any(|n| n == expected),
            "missing {expected} in {names:?}"
        );
    }
}

#[test]
fn transform_offers_the_engine_step_types() {
    let names: Vec<String> = build_transform_type_specs(true)
        .into_iter()
        .map(|s| s.r#type)
        .collect();
    for expected in ["none", "arithmetic", "concat", "authenticate", "send_email"] {
        assert!(
            names.iter().any(|n| n == expected),
            "missing {expected} in {names:?}"
        );
    }
}

/// `none` means "no transform", so offering fields for it would be a bug.
#[test]
fn the_none_transform_takes_no_fields() {
    let specs = build_transform_type_specs(true);
    assert!(find(&specs, "none").fields.is_empty());
}

// --- individual shapes the UI depends on ----------------------------------

#[test]
fn arithmetic_exposes_both_operands_and_an_operator() {
    let specs = build_transform_type_specs(true);
    let arithmetic = find(&specs, "arithmetic");

    assert!(field(arithmetic, "target").required);
    assert!(field(arithmetic, "left.kind").required);

    let op = field(arithmetic, "op");
    assert!(op.required);
    let options = op.options.as_ref().unwrap();
    for expected in ["add", "sub", "mul", "div"] {
        assert!(
            options.iter().any(|o| o == expected),
            "missing op {expected}"
        );
    }

    // Both operands offer the same shape, so the UI can render one component twice.
    for side in ["left", "right"] {
        for suffix in ["kind", "field", "value"] {
            field(arithmetic, &format!("{side}.{suffix}"));
        }
    }
}

#[test]
fn operand_kinds_agree_between_the_two_sides() {
    let specs = build_transform_type_specs(true);
    let arithmetic = find(&specs, "arithmetic");

    assert_eq!(
        field(arithmetic, "left.kind").options,
        field(arithmetic, "right.kind").options,
        "the two operands must accept the same kinds"
    );
}

#[test]
fn every_type_spec_serialises_for_the_frontend() {
    for (group, specs) in all_specs() {
        let json = serde_json::to_value(&specs).unwrap();
        let array = json
            .as_array()
            .unwrap_or_else(|| panic!("{group} is not an array"));
        assert_eq!(array.len(), specs.len());

        for entry in array {
            assert!(entry.get("type").is_some(), "{group} entry has no type");
            assert!(entry.get("fields").is_some(), "{group} entry has no fields");
        }
    }
}

#[test]
fn required_fields_come_with_a_usable_type() {
    // A required enum with no options would render an unfillable form.
    for (group, specs) in all_specs() {
        for spec in &specs {
            for f in spec.fields.iter().filter(|f| f.required) {
                if f.r#type == "enum" {
                    assert!(
                        f.options.as_ref().is_some_and(|o| !o.is_empty()),
                        "{group}/{}/{} is required but unfillable",
                        spec.r#type,
                        f.name
                    );
                }
            }
        }
    }
}
