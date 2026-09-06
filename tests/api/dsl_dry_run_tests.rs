#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! The `/dsl/dry-run` endpoint.
//!
//! The promise being tested is narrow and important: the program really runs —
//! transforms apply, mappings resolve, lookups return real data — and nothing
//! is persisted. A dry-run that quietly skipped half the transforms, or that
//! left rows behind, would be worse than none at all, because callers are told
//! to trust it.

use actix_web::{http::StatusCode, test};
use serial_test::serial;

use super::users::common::{get_auth_token, setup_test_app};

const DRY_RUN: &str = "/admin/api/v1/dsl/dry-run";

/// A single arithmetic step with an API source and a format target.
fn arithmetic_program() -> serde_json::Value {
    serde_json::json!([{
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
async fn test_dry_run_applies_transforms_and_reports_the_trace() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::post()
        .uri(DRY_RUN)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&serde_json::json!({
            "steps": arithmetic_program(),
            "input": { "price": 100 }
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let steps = body["data"]["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("expected a steps array in {body}"));

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["step_index"], 0);
    assert_eq!(
        steps[0]["produced"]["total"], 119.0,
        "the transform must actually have run: {}",
        steps[0]
    );
    assert_eq!(
        steps[0]["effect"], "no_side_effect",
        "a format/api target persists nothing"
    );
}

#[serial]
#[tokio::test]
async fn test_dry_run_reports_nothing_would_be_written_for_a_read_only_program() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::post()
        .uri(DRY_RUN)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&serde_json::json!({
            "steps": arithmetic_program(),
            "input": { "price": 100 }
        }))
        .to_request();

    let body: serde_json::Value = test::read_body_json(test::call_service(&app, req).await).await;

    assert_eq!(
        body["data"]["would_write"]
            .as_array()
            .unwrap_or_else(|| panic!("expected would_write in {body}"))
            .len(),
        0,
        "a program with no entity target writes nothing"
    );
}

#[serial]
#[tokio::test]
async fn test_dry_run_rejects_an_invalid_program_before_executing() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    // PreviousStep is illegal in step 0.
    let req = test::TestRequest::post()
        .uri(DRY_RUN)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&serde_json::json!({
            "steps": [{
                "from": { "type": "previous_step", "mapping": {} },
                "transform": { "type": "none" },
                "to": { "type": "format", "output": { "mode": "api" },
                        "format": { "format_type": "json", "options": {} },
                        "mapping": {} }
            }],
            "input": {}
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[serial]
#[tokio::test]
async fn test_dry_run_locates_a_malformed_step() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::post()
        .uri(DRY_RUN)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&serde_json::json!({
            "steps": [{ "from": { "type": "not_a_source" } }],
            "input": {}
        }))
        .to_request();

    let body: serde_json::Value = test::read_body_json(test::call_service(&app, req).await).await;

    assert_eq!(
        body["violations"][0]["json_path"], "steps[0].from.type",
        "dry-run must locate failures as precisely as validate does: {body}"
    );
}

#[serial]
#[tokio::test]
async fn test_dry_run_requires_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    let req = test::TestRequest::post()
        .uri(DRY_RUN)
        .set_json(&serde_json::json!({ "steps": [], "input": {} }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// The guarantee the authoring loop rests on: iterating freely creates nothing.
#[serial]
#[tokio::test]
async fn test_dry_run_leaves_no_trace_in_the_database() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entities_registry")
        .fetch_one(&pool.pool)
        .await
        .unwrap_or(0);

    // Run the same program several times, as an assistant iterating would.
    for _ in 0..3 {
        let req = test::TestRequest::post()
            .uri(DRY_RUN)
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(&serde_json::json!({
                "steps": arithmetic_program(),
                "input": { "price": 100 }
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entities_registry")
        .fetch_one(&pool.pool)
        .await
        .unwrap_or(0);

    assert_eq!(
        before, after,
        "repeated dry-runs must not accumulate rows — this is the whole promise"
    );
}
