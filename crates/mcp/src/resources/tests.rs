#![allow(clippy::expect_used)]

use super::*;
use serde_json::json;

fn field(name: &str, ty: &str, required: bool, options: Option<Vec<String>>) -> DslFieldSpec {
    DslFieldSpec {
        name: name.to_string(),
        r#type: ty.to_string(),
        required,
        options,
    }
}

fn catalogue() -> Vec<(&'static str, DslOptionsAndExamplesResponse)> {
    vec![(
        "to",
        DslOptionsAndExamplesResponse {
            types: vec![
                DslTypeSpec {
                    r#type: "entity".to_string(),
                    fields: vec![
                        field("entity_definition", "string", true, None),
                        field(
                            "mode",
                            "enum",
                            true,
                            Some(vec![
                                "create".to_string(),
                                "update".to_string(),
                                "create_or_update".to_string(),
                            ]),
                        ),
                        field("update_key", "string", false, None),
                    ],
                },
                DslTypeSpec {
                    r#type: "next_step".to_string(),
                    fields: vec![],
                },
            ],
            examples: vec![json!({ "type": "next_step", "mapping": {} })],
        },
    )]
}

#[test]
fn every_type_appears() {
    let rendered = render_dsl_reference(&catalogue());
    assert!(rendered.contains("`entity`"), "{rendered}");
    assert!(rendered.contains("`next_step`"), "{rendered}");
}

#[test]
fn fields_carry_their_type_and_requiredness() {
    let rendered = render_dsl_reference(&catalogue());
    assert!(
        rendered.contains("`entity_definition` (string, required)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("`update_key` (string, optional)"),
        "{rendered}"
    );
}

#[test]
fn enumerated_values_are_listed() {
    // The single most useful thing in the reference: it turns a guess into a
    // choice, and is what stops a model inventing a mode like "upsert".
    let rendered = render_dsl_reference(&catalogue());
    assert!(
        rendered.contains("create_or_update"),
        "legal values must be spelled out: {rendered}"
    );
}

#[test]
fn a_type_with_no_fields_says_so_rather_than_looking_truncated() {
    let rendered = render_dsl_reference(&catalogue());
    assert!(rendered.contains("Takes no fields."), "{rendered}");
}

#[test]
fn the_struct_serialized_example_is_included() {
    // These come from the real DSL types, so they are the most trustworthy
    // thing in the document.
    let rendered = render_dsl_reference(&catalogue());
    assert!(rendered.contains("\"type\": \"next_step\""), "{rendered}");
}

#[test]
fn an_empty_catalogue_renders_without_panicking() {
    let rendered = render_dsl_reference(&[(
        "to",
        DslOptionsAndExamplesResponse {
            types: vec![],
            examples: vec![],
        },
    )]);
    assert!(rendered.contains("No types available"), "{rendered}");
}

#[test]
fn the_rules_cover_the_ordering_constraints_a_model_gets_wrong() {
    let rules = validation_rules();
    for expected in [
        "previous_step",
        "next_step",
        "trigger",
        "at least one step",
        "Division by zero",
    ] {
        assert!(rules.contains(expected), "rules omit {expected}");
    }
}

#[test]
fn the_rules_point_at_the_dry_run() {
    // Validation cannot catch mapping and type errors; the rules must say so,
    // or a model will stop at a green validate.
    assert!(validation_rules().contains("test_workflow"));
}

// ── the advertised descriptors ──────────────────────────────────────────────

#[test]
fn descriptors_cover_both_documents_and_nothing_else() {
    let uris: Vec<String> = descriptors().into_iter().map(|r| r.uri).collect();
    assert_eq!(uris, vec![DSL_REFERENCE_URI, DSL_RULES_URI]);
}

#[test]
fn descriptors_are_markdown_and_named_for_a_reader() {
    // A client renders these in a picker. A resource with no name or no
    // description is one a person cannot choose between.
    for resource in descriptors() {
        assert_eq!(resource.mime_type.as_deref(), Some("text/markdown"));
        assert!(!resource.name.is_empty(), "{} has no name", resource.uri);
        assert!(
            resource.description.is_some(),
            "{} has no description",
            resource.uri
        );
    }
}
