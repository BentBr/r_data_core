#![allow(clippy::unwrap_used)]

//! Guards that every DSL variant the engine accepts is advertised by the
//! `/dsl/*/options` catalogue.
//!
//! The option specs are hand-maintained, so a variant added to the engine can
//! silently fail to appear here. That is worse than a missing feature: the
//! catalogue is the vocabulary the admin UI builds its forms from, and the
//! authoritative DSL reference an MCP client teaches a model from. A variant
//! absent from it is not merely undiscoverable — consumers are actively told
//! it does not exist.
//!
//! `tests.rs` next door already checks a few names, but from lists written by
//! hand, which drift the same way the catalogue does. The difference here is
//! the `variant_names` helpers below: each contains a `match` over the real
//! engine enum, and match exhaustiveness is a **compile error**. Adding a
//! variant to `FromDef`, `ToDef` or `Transform` therefore breaks this file
//! until the expected list is updated in the same change — which is the point.

use r_data_core_workflow::dsl::{FromDef, ToDef, Transform};

use super::{build_from_type_specs, build_to_type_specs, build_transform_type_specs};

/// Every `FromDef` variant, spelled as the DSL spells it.
fn from_variant_names() -> &'static [&'static str] {
    #[allow(dead_code)] // Exists for its exhaustiveness check, never called.
    const fn tripwire(v: &FromDef) -> &'static str {
        match v {
            FromDef::Format { .. } => "format",
            FromDef::Entity { .. } => "entity",
            FromDef::PreviousStep { .. } => "previous_step",
            FromDef::Trigger { .. } => "trigger",
        }
    }
    &["format", "entity", "previous_step", "trigger"]
}

/// Every `ToDef` variant, spelled as the DSL spells it.
fn to_variant_names() -> &'static [&'static str] {
    #[allow(dead_code)]
    const fn tripwire(v: &ToDef) -> &'static str {
        match v {
            ToDef::Format { .. } => "format",
            ToDef::Entity { .. } => "entity",
            ToDef::NextStep { .. } => "next_step",
            ToDef::Email { .. } => "email",
        }
    }
    &["format", "entity", "next_step", "email"]
}

/// Every `Transform` variant, spelled as the DSL spells it.
fn transform_variant_names() -> &'static [&'static str] {
    #[allow(dead_code)]
    const fn tripwire(v: &Transform) -> &'static str {
        match v {
            Transform::None => "none",
            Transform::Arithmetic(_) => "arithmetic",
            Transform::Concat(_) => "concat",
            Transform::ResolveEntityPath(_) => "resolve_entity_path",
            Transform::BuildPath(_) => "build_path",
            Transform::GetOrCreateEntity(_) => "get_or_create_entity",
            Transform::Authenticate(_) => "authenticate",
            Transform::SendEmail(_) => "send_email",
        }
    }
    &[
        "none",
        "arithmetic",
        "concat",
        "resolve_entity_path",
        "build_path",
        "get_or_create_entity",
        "authenticate",
        "send_email",
    ]
}

fn assert_covers(group: &str, advertised: &[String], expected: &[&str]) {
    let missing: Vec<&&str> = expected
        .iter()
        .filter(|want| !advertised.iter().any(|got| got == *want))
        .collect();

    assert!(
        missing.is_empty(),
        "/dsl/{group}/options does not advertise {missing:?}. The engine accepts \
         these, so consumers are being told they are illegal: the admin UI cannot \
         offer them and an MCP client teaches a model they do not exist. Add them \
         to the {group} option builder. Advertised: {advertised:?}"
    );
}

/// The mail-gated variants (`to: email`, `transform: send_email`) are absent
/// unless a mailer is configured, so the catalogue is built with gating on.
fn advertised(specs: Vec<super::super::super::models::DslTypeSpec>) -> Vec<String> {
    specs.into_iter().map(|s| s.r#type).collect()
}

#[test]
fn from_options_advertise_every_engine_variant() {
    assert_covers(
        "from",
        &advertised(build_from_type_specs()),
        from_variant_names(),
    );
}

#[test]
fn to_options_advertise_every_engine_variant() {
    assert_covers(
        "to",
        &advertised(build_to_type_specs(true)),
        to_variant_names(),
    );
}

#[test]
fn transform_options_advertise_every_engine_variant() {
    assert_covers(
        "transform",
        &advertised(build_transform_type_specs(true)),
        transform_variant_names(),
    );
}

/// The catalogue must not advertise a type the engine cannot parse either —
/// that sends a model or a form-builder down a path that fails at runtime.
#[test]
fn options_advertise_nothing_the_engine_does_not_accept() {
    for (group, advertised, expected) in [
        (
            "from",
            advertised(build_from_type_specs()),
            from_variant_names(),
        ),
        (
            "to",
            advertised(build_to_type_specs(true)),
            to_variant_names(),
        ),
        (
            "transform",
            advertised(build_transform_type_specs(true)),
            transform_variant_names(),
        ),
    ] {
        let unknown: Vec<&String> = advertised
            .iter()
            .filter(|got| !expected.iter().any(|want| *want == got.as_str()))
            .collect();

        assert!(
            unknown.is_empty(),
            "/dsl/{group}/options advertises {unknown:?}, which the engine has no \
             variant for. Consumers will build steps that fail to parse."
        );
    }
}
