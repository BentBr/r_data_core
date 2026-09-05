#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! The `/dsl/*/options` endpoints the workflow editor reads its step
//! vocabulary from, plus `/dsl/validate`.

use actix_web::{http::StatusCode, test};
use serial_test::serial;

use super::users::common::{get_auth_token, setup_test_app};

const FROM: &str = "/admin/api/v1/dsl/from/options";
const TO: &str = "/admin/api/v1/dsl/to/options";
const TRANSFORM: &str = "/admin/api/v1/dsl/transform/options";
const VALIDATE: &str = "/admin/api/v1/dsl/validate";

/// Every options endpoint answers with `{ types: [ { type, fields } ] }`.
#[serial]
#[tokio::test]
async fn test_options_endpoints_return_typed_specs() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    for uri in [FROM, TO, TRANSFORM] {
        let req = test::TestRequest::get()
            .uri(uri)
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK, "{uri} should answer 200");

        let body: serde_json::Value = test::read_body_json(resp).await;
        let types = body["data"]["types"]
            .as_array()
            .unwrap_or_else(|| panic!("{uri} returned no types array"));
        assert!(!types.is_empty(), "{uri} returned an empty vocabulary");

        for entry in types {
            assert!(entry["type"].is_string(), "{uri} entry has no type");
            assert!(entry["fields"].is_array(), "{uri} entry has no fields");
        }
    }
}

/// The editor keys its step forms off these names, so the engine's step types
/// have to be present.
#[serial]
#[tokio::test]
async fn test_options_cover_the_engine_step_types() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let names = |body: &serde_json::Value| -> Vec<String> {
        body["data"]["types"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|t| t["type"].as_str().map(str::to_string))
            .collect()
    };

    for (uri, expected) in [
        (FROM, vec!["format", "entity"]),
        (TO, vec!["format", "entity"]),
        (
            TRANSFORM,
            vec!["none", "arithmetic", "concat", "authenticate"],
        ),
    ] {
        let req = test::TestRequest::get()
            .uri(uri)
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
        let got = names(&body);

        for want in expected {
            assert!(
                got.iter().any(|n| n == want),
                "{uri} is missing {want}: {got:?}"
            );
        }
    }
}

/// These endpoints describe internal workflow capability and must not be
/// readable without a token.
#[serial]
#[tokio::test]
async fn test_options_endpoints_require_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    for uri in [FROM, TO, TRANSFORM] {
        let req = test::TestRequest::get().uri(uri).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{uri} must reject an anonymous caller"
        );
    }
}

#[serial]
#[tokio::test]
async fn test_validate_accepts_a_well_formed_step() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let steps = serde_json::json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": {
                    "source_type": "uri",
                    "config": { "uri": "http://example.com/data.csv" },
                    "auth": { "type": "none" }
                },
                "format": {
                    "format_type": "csv",
                    "options": { "has_header": true, "delimiter": "," }
                },
                "mapping": { "price": "price" }
            },
            "transform": { "type": "none" },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "price": "entity.total" }
            }
        }]
    });

    let req = test::TestRequest::post()
        .uri(VALIDATE)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&steps)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(
        body["data"]["valid"], true,
        "well-formed DSL should validate"
    );
}

#[serial]
#[tokio::test]
async fn test_validate_rejects_an_unknown_step_type() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::post()
        .uri(VALIDATE)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&serde_json::json!({
            "steps": [{ "from": { "type": "not_a_real_source" } }]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_ne!(
        resp.status(),
        StatusCode::OK,
        "an unknown step type must not validate"
    );
}

/// A malformed step must say which step failed and why.
///
/// This previously collapsed every parse failure into a bare
/// "Invalid DSL steps format", discarding serde's message — which is the one
/// thing that names the offending field and lists the legal alternatives.
/// Callers were left with no way to locate the problem.
#[serial]
#[tokio::test]
async fn test_validate_reports_which_step_failed_and_why() {
    let (app, pool, _uuid) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let req = test::TestRequest::post()
        .uri(VALIDATE)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&serde_json::json!({
            "steps": [
                {
                    "from": { "type": "trigger", "mapping": {} },
                    "transform": { "type": "none" },
                    "to": { "type": "next_step", "mapping": {} }
                },
                { "from": { "type": "definitely_not_a_source" } }
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let violations = body["violations"]
        .as_array()
        .unwrap_or_else(|| panic!("expected violations in {body}"));

    assert_eq!(
        violations.len(),
        1,
        "only the second step is malformed, got {violations:?}"
    );
    assert_eq!(
        violations[0]["field"], "steps[1]",
        "the violation must name the offending step, got {violations:?}"
    );

    let message = violations[0]["message"]
        .as_str()
        .unwrap_or_else(|| panic!("violation has no message: {violations:?}"));
    assert!(
        message.contains("definitely_not_a_source"),
        "serde's message must survive so the caller sees the bad value: {message}"
    );
}

#[serial]
#[tokio::test]
async fn test_validate_requires_authentication() {
    let (app, _pool, _uuid) = setup_test_app().await.unwrap();

    let req = test::TestRequest::post()
        .uri(VALIDATE)
        .set_json(&serde_json::json!({ "steps": [] }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
