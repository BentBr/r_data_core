#![allow(clippy::expect_used)]

//! The prompts are text, so these tests assert on content. That is the right
//! call here: a prompt that omits the dry-run step has failed at its only job.

use super::*;

#[test]
fn the_authoring_prompt_orders_the_loop_correctly() {
    // Discovery before drafting, validation before saving, dry-run before the
    // real run. Out of order, the sequence stops preventing anything.
    let positions: Vec<usize> = [
        "dsl_options",
        "get_entity_definition",
        "validate_dsl",
        "test_workflow",
        "create_workflow",
        "run_workflow",
    ]
    .iter()
    .map(|needle| {
        AUTHOR_WORKFLOW
            .find(needle)
            .unwrap_or_else(|| panic!("the authoring prompt never mentions {needle}"))
    })
    .collect();

    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(
        positions, sorted,
        "the steps must appear in the order they should be performed"
    );
}

#[test]
fn the_authoring_prompt_insists_on_the_dry_run() {
    // A model that stops at a green validate discovers its mapping errors by
    // writing bad data.
    assert!(AUTHOR_WORKFLOW.contains("Never skip"));
    assert!(AUTHOR_WORKFLOW.contains("persists nothing"));
}

#[test]
fn the_authoring_prompt_forbids_guessing_field_names() {
    assert!(
        AUTHOR_WORKFLOW.contains("Guessing them"),
        "the commonest cause of a workflow that validates then fails"
    );
}

#[test]
fn the_authoring_prompt_says_not_to_rely_on_remembered_syntax() {
    assert!(AUTHOR_WORKFLOW.contains("remembered syntax"));
}

#[test]
fn the_authoring_prompt_creates_disabled() {
    assert!(AUTHOR_WORKFLOW.contains("stays disabled"));
}

#[test]
fn the_debug_prompt_starts_from_the_logs_not_the_program() {
    let logs = DEBUG_FAILING_RUN
        .find("get_run_logs")
        .expect("must mention the logs");
    let program = DEBUG_FAILING_RUN
        .find("get_workflow")
        .expect("must mention the program");
    assert!(
        logs < program,
        "the logs name the failing step; reading the whole program first \
         wastes the cheapest clue available"
    );
}

#[test]
fn the_debug_prompt_confirms_a_fix_before_applying_it() {
    let test = DEBUG_FAILING_RUN
        .find("test_workflow")
        .expect("must dry-run the fix");
    let update = DEBUG_FAILING_RUN
        .find("update_workflow")
        .expect("must apply the fix");
    assert!(test < update, "confirm before applying");
}

#[test]
fn the_debug_prompt_mentions_restoring_a_working_version() {
    // Often faster than reconstructing the program by hand.
    assert!(DEBUG_FAILING_RUN.contains("restore_workflow_version"));
}

#[test]
fn the_debug_prompt_says_where_cast_failures_usually_live() {
    assert!(DEBUG_FAILING_RUN.contains("not in the destination"));
}
