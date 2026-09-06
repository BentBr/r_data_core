#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! The DSL reference a model reads before writing a program.
//!
//! Rendered from the live `/dsl/*/options` catalogue, **not** from
//! `docs/DSL.md`. That distinction is the whole point. The markdown documents
//! roughly a third of the language — three of eight transforms, no `email`
//! target, no post-run hooks — so teaching from it would tell a model that
//! most of the DSL does not exist. The catalogue is generated from the same
//! structs the executor consumes and therefore cannot drift; a test in
//! `crates/api` fails the build if a variant is ever missing from it.
//!
//! Output is compact Markdown. The reader is a model with a finite context,
//! so every line has to earn its place: types, their fields, requiredness,
//! enumerated values, and one worked example per group.

use std::fmt::Write as _;

use r_data_core_core::dto::dsl::{DslFieldSpec, DslOptionsAndExamplesResponse, DslTypeSpec};

/// URI of the generated DSL reference.
pub const DSL_REFERENCE_URI: &str = "rdatacore://dsl/reference";
/// URI of the hand-written rules the catalogue cannot express.
pub const DSL_RULES_URI: &str = "rdatacore://dsl/validation-rules";

/// Render the catalogue for every DSL group into one reference document.
#[must_use]
pub fn render_dsl_reference(groups: &[(&str, DslOptionsAndExamplesResponse)]) -> String {
    let mut out = String::from(
        "# RDataCore DSL reference\n\n\
         Generated from the running engine, so it is always accurate for this \
         instance. A DSL program is `{ \"steps\": [ { \"from\": …, \"transform\": …, \
         \"to\": … } ] }`.\n",
    );

    for (group, options) in groups {
        let _ = write!(out, "\n## {group}\n");
        if options.types.is_empty() {
            out.push_str("\n_No types available._\n");
            continue;
        }
        for spec in &options.types {
            render_type(&mut out, spec);
        }
        render_examples(&mut out, group, &options.examples);
    }

    out
}

fn render_type(out: &mut String, spec: &DslTypeSpec) {
    let _ = write!(out, "\n### `{}`\n", spec.r#type);
    if spec.fields.is_empty() {
        out.push_str("\nTakes no fields.\n");
        return;
    }
    out.push('\n');
    for field in &spec.fields {
        render_field(out, field);
    }
}

fn render_field(out: &mut String, field: &DslFieldSpec) {
    let requirement = if field.required {
        "required"
    } else {
        "optional"
    };
    let _ = write!(out, "- `{}` ({}, {requirement})", field.name, field.r#type);
    // Enumerated values are the single most useful thing here: they turn a
    // guess into a choice.
    if let Some(options) = field.options.as_ref().filter(|o| !o.is_empty()) {
        let _ = write!(out, " — one of: {}", options.join(", "));
    }
    out.push('\n');
}

fn render_examples(out: &mut String, group: &str, examples: &[serde_json::Value]) {
    let Some(example) = examples.first() else {
        return;
    };
    let _ = write!(
        out,
        "\n#### Example `{group}`\n\n```json\n{}\n```\n",
        serde_json::to_string_pretty(example).unwrap_or_else(|_| example.to_string())
    );
}

/// Rules the option catalogue cannot express.
///
/// The catalogue describes shapes; these are ordering and typing constraints
/// the validator enforces, and they are the ones a model most often gets
/// wrong. Kept hand-written and short.
#[must_use]
pub const fn validation_rules() -> &'static str {
    "# DSL rules the option catalogue cannot express

## Step ordering
- `from: previous_step` is invalid in step 0 — there is no previous step to read.
- `to: next_step` is invalid in the last step — there is nothing to pass to.
- `from: trigger` is only valid in step 0.
- A program must contain at least one step.

## Mappings
- `from.mapping` maps a source field to a normalized name: `{ \"source\": \"normalized\" }`.
- `to.mapping` maps the other way: `{ \"normalized\": \"destination\" }`.
- An empty mapping `{}` passes every field through unchanged.
- Field names must match `^[A-Za-z_][A-Za-z0-9_.]*$`.

## Type casting
- Arithmetic casts numeric strings: `\"123.45\"` becomes `123.45`. `\"abc\"` is an error.
- Concat casts numbers to strings, and an integral float loses its `.0`: `123.0` becomes `\"123\"`.
- A null operand in arithmetic or concat is an error, not a zero or an empty string.
- Division by zero is rejected.

## Getting it right first time
1. Read this and the option catalogue before drafting.
2. For an entity target, call `get_entity_definition` — never guess field names.
3. Call `validate_dsl`, then `test_workflow`. The dry-run catches mapping and
   type errors that validation cannot see.
"
}

#[cfg(test)]
mod tests;
