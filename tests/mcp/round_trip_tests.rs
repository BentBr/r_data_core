#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The MCP tools against a real `RDataCore`.
//!
//! What these add over the crate's own wiremock tests: proof that the paths,
//! response envelopes and permission strings the client assumes are the ones
//! the server actually produces. A mock cannot tell you that, and the
//! assumption has been wrong before.

use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;
use serial_test::serial;

use r_data_core_mcp::tools::discover::{DslOptionsParams, ListWorkflowsParams};
use r_data_core_mcp::tools::execute::TestWorkflowParams;

use super::harness::{text_of, McpTestStack};

/// A single arithmetic step: no entities, so it needs no fixtures.
fn arithmetic_steps() -> serde_json::Value {
    json!([{
        "from": {
            "type": "format",
            "source": { "source_type": "api", "config": {} },
            "format": { "format_type": "json", "options": {} },
            "mapping": { "price": "price" }
        },
        "transform": {
            "type": "arithmetic",
            "target": "total",
            "left": { "kind": "field", "field": "price" },
            "op": "mul",
            "right": { "kind": "const", "value": 1.19 }
        },
        "to": {
            "type": "format",
            "output": { "mode": "api" },
            "format": { "format_type": "json", "options": {} },
            "mapping": {}
        }
    }])
}

#[serial]
#[tokio::test]
async fn the_client_can_reach_a_real_server() {
    let stack = McpTestStack::start().await;
    let result = stack
        .tools()
        .await
        .list_workflows(Parameters(ListWorkflowsParams {
            limit: None,
            offset: None,
            enabled: None,
        }))
        .await
        .expect("tool call");

    assert_ne!(
        result.is_error,
        Some(true),
        "listing against a real server failed: {}",
        text_of(&result)
    );
}

#[serial]
#[tokio::test]
async fn the_dsl_catalogue_comes_back_populated() {
    // The reference a model writes DSL from. An empty or unreachable
    // catalogue would leave it guessing.
    let stack = McpTestStack::start().await;
    let tools = stack.tools().await;

    for kind in ["from", "to", "transform"] {
        let result = tools
            .dsl_options(Parameters(DslOptionsParams {
                kind: kind.to_string(),
            }))
            .await
            .expect("tool call");

        let text = text_of(&result);
        assert_ne!(result.is_error, Some(true), "{kind}: {text}");
        assert!(
            text.contains("\"types\""),
            "{kind} returned no types: {text}"
        );
    }
}

#[serial]
#[tokio::test]
async fn the_catalogue_includes_the_variants_the_guard_added() {
    // previous_step, trigger, next_step and the three path transforms were
    // missing from the catalogue until the drift guard caught them. This
    // checks they reach a model through the full stack, not just the builder.
    let stack = McpTestStack::start().await;
    let tools = stack.tools().await;

    let from = text_of(
        &tools
            .dsl_options(Parameters(DslOptionsParams {
                kind: "from".to_string(),
            }))
            .await
            .expect("tool call"),
    );
    assert!(from.contains("previous_step"), "{from}");
    assert!(from.contains("trigger"), "{from}");

    let transform = text_of(
        &tools
            .dsl_options(Parameters(DslOptionsParams {
                kind: "transform".to_string(),
            }))
            .await
            .expect("tool call"),
    );
    for expected in ["resolve_entity_path", "build_path", "get_or_create_entity"] {
        assert!(
            transform.contains(expected),
            "missing {expected}: {transform}"
        );
    }
}

#[serial]
#[tokio::test]
async fn a_dry_run_executes_the_transform_and_writes_nothing() {
    let stack = McpTestStack::start().await;

    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entities_registry")
        .fetch_one(&stack.pool.pool)
        .await
        .unwrap_or(0);

    let result = stack
        .tools()
        .await
        .test_workflow(Parameters(TestWorkflowParams {
            steps: Some(arithmetic_steps()),
            uuid: None,
            input: json!({ "price": 100 }),
        }))
        .await
        .expect("tool call");

    let text = text_of(&result);
    assert_ne!(result.is_error, Some(true), "{text}");
    assert!(
        text.contains("119"),
        "the transform must have actually run: {text}"
    );

    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entities_registry")
        .fetch_one(&stack.pool.pool)
        .await
        .unwrap_or(0);
    assert_eq!(before, after, "a dry-run must persist nothing");
}

#[serial]
#[tokio::test]
async fn an_invalid_program_comes_back_located() {
    // The end-to-end proof of the teaching-error chain: the server produces a
    // JSON path and legal values, the client parses them, and the tool renders
    // them into text a model can act on.
    let stack = McpTestStack::start().await;

    let result = stack
        .tools()
        .await
        .test_workflow(Parameters(TestWorkflowParams {
            steps: Some(json!([{ "from": { "type": "not_a_real_source" } }])),
            uuid: None,
            input: json!({}),
        }))
        .await
        .expect("tool call");

    let text = text_of(&result);
    assert_eq!(result.is_error, Some(true), "{text}");
    assert!(
        text.contains("steps[0].from.type"),
        "the path must survive the whole chain: {text}"
    );
    assert!(
        text.contains("previous_step"),
        "the legal values must survive too: {text}"
    );
}

#[serial]
#[tokio::test]
async fn permissions_resolve_against_the_live_server() {
    // The binary resolves permissions at startup and narrows its tool router
    // from them. If the server's strings and the registry's expectations ever
    // diverge, every tool silently disappears — so assert the surface is not
    // empty and includes a known read tool.
    let stack = McpTestStack::start().await;
    let visible = stack.tools().await.visible_tool_names();

    assert!(
        !visible.is_empty(),
        "an admin key resolved to no usable tools, which means the permission \
         strings no longer match the server"
    );
    assert!(visible.contains(&"list_workflows"), "got {visible:?}");
}
