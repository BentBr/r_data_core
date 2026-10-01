#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix test app is Rc-based and held across awaits

//! The credential the public endpoint checks lives at `config.provider_auth`,
//! beside the DSL rather than inside it, so the DSL parse never sees it. These
//! prove it is policed on the way in.

use actix_web::test;
use serial_test::serial;
use uuid::Uuid;

use super::common::setup_app_with_entities;

/// A 32-character key: exactly the floor, so the test also pins the boundary.
const LONG_KEY: &str = "0123456789abcdef0123456789abcdef";

fn config_with_key(key: &str) -> serde_json::Value {
    serde_json::json!({
        "provider_auth": {
            "type": "pre_shared_key",
            "key": key,
            "location": "header",
            "field_name": "X-Pre-Shared-Key"
        },
        "steps": [{
            "from": {
                "type": "format",
                "source": {
                    "source_type": "uri",
                    "config": { "uri": "http://example.com/data.csv" },
                    "auth": { "type": "none" }
                },
                "format": { "format_type": "csv", "options": { "has_header": true, "delimiter": "," } },
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
    })
}

fn create_body(key: &str) -> serde_json::Value {
    serde_json::json!({
        "name": format!("psk-wf-{}", Uuid::now_v7().simple()),
        "description": "provider auth validation",
        "kind": "provider",
        "enabled": true,
        "schedule_cron": serde_json::Value::Null,
        "config": config_with_key(key),
        "versioning_disabled": false,
    })
}

#[actix_web::test]
#[serial]
async fn test_creating_a_workflow_with_a_short_provider_key_is_rejected() -> anyhow::Result<()> {
    let (app, _pool, token, _) = setup_app_with_entities().await?;

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/workflows")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(create_body("short"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    assert!(
        status.is_client_error(),
        "a 5-character public key must be refused, got {status}"
    );

    let body = String::from_utf8_lossy(&test::read_body(resp).await).into_owned();
    assert!(
        body.contains("32"),
        "the error must tell the admin the required length, got: {body}"
    );

    Ok(())
}

#[actix_web::test]
#[serial]
async fn test_updating_a_workflow_to_a_short_provider_key_is_rejected() -> anyhow::Result<()> {
    let (app, _pool, token, _) = setup_app_with_entities().await?;

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/workflows")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(create_body(LONG_KEY))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "a 32-character key must be accepted, got {}",
        resp.status()
    );
    let created: serde_json::Value = test::read_body_json(resp).await;
    let uuid = created
        .pointer("/data/uuid")
        .or_else(|| created.get("uuid"))
        .and_then(serde_json::Value::as_str)
        .expect("create response must carry the new uuid")
        .to_string();

    // A fresh body carrying everything the update needs, differing only in the
    // key length - so a refusal can only be about the key.
    let short = create_body("short");
    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/workflows/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(&short)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_client_error(),
        "shortening the key on update must be refused, got {}",
        resp.status()
    );

    // Refused is not enough: nothing may have been written either.
    let req = test::TestRequest::get()
        .uri(&format!("/admin/api/v1/workflows/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let stored: serde_json::Value = test::read_body_json(test::call_service(&app, req).await).await;
    let persisted = serde_json::to_string(&stored)?;
    assert!(
        persisted.contains(LONG_KEY),
        "the original key must still be in place after a refused update"
    );

    Ok(())
}

/// End-to-end proof the floor does not break the happy path.
#[actix_web::test]
#[serial]
async fn test_a_long_provider_key_is_accepted_and_authenticates() -> anyhow::Result<()> {
    let (app, _pool, token, _) = setup_app_with_entities().await?;

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/workflows")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(create_body(LONG_KEY))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "got {}", resp.status());
    let created: serde_json::Value = test::read_body_json(resp).await;
    let uuid = created
        .pointer("/data/uuid")
        .or_else(|| created.get("uuid"))
        .and_then(serde_json::Value::as_str)
        .expect("create response must carry the new uuid")
        .to_string();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{uuid}"))
        .insert_header(("X-Pre-Shared-Key", LONG_KEY))
        .to_request();
    let status = test::call_service(&app, req).await.status();
    assert_ne!(
        status.as_u16(),
        401,
        "the accepted key must authenticate against the public endpoint"
    );

    Ok(())
}
