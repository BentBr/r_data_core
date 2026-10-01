#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix test app is Rc-based and held across awaits

//! Opt-in per-workflow throttling on the public API.
//!
//! The limit is deliberately not global: one workflow's traffic must never
//! consume another workflow's budget, and one caller must never consume
//! another caller's.

use actix_web::{http::StatusCode, test};
use serial_test::serial;
use std::net::SocketAddr;
use uuid::Uuid;

use super::common::{create_consumer_workflow, create_provider_workflow, setup_app_with_entities};

/// A provider config that exposes JSON over the API, optionally rate limited.
fn provider_config(rate_limit: Option<serde_json::Value>) -> serde_json::Value {
    let mut config = serde_json::json!({
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

    if let Some(limit) = rate_limit {
        config["rate_limit"] = limit;
    }
    config
}

fn limited(max: u32, minutes: u32) -> serde_json::Value {
    serde_json::json!({ "enabled": true, "max_requests": max, "window_minutes": minutes })
}

async fn call<S>(app: &S, ip: SocketAddr, workflow: Uuid) -> StatusCode
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{workflow}"))
        .peer_addr(ip)
        .to_request();
    test::call_service(app, req).await.status()
}

/// Without a configured limit nothing is throttled, however hard it is hit.
#[actix_web::test]
#[serial]
async fn test_a_workflow_without_a_limit_is_never_throttled() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;
    let creator = r_data_core_test_support::create_test_admin_user(&pool.pool).await?;
    let uuid = create_provider_workflow(&pool.pool, creator, provider_config(None)).await?;
    let ip: SocketAddr = "203.0.113.90:5000".parse()?;

    for attempt in 1..=20 {
        assert_ne!(
            call(&app, ip, uuid).await,
            StatusCode::TOO_MANY_REQUESTS,
            "attempt {attempt} throttled a workflow with no limit configured"
        );
    }

    Ok(())
}

#[actix_web::test]
#[serial]
async fn test_an_enabled_limit_returns_429_after_the_budget() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;
    let creator = r_data_core_test_support::create_test_admin_user(&pool.pool).await?;
    let uuid = create_provider_workflow(&pool.pool, creator, provider_config(Some(limited(3, 60))))
        .await?;
    let ip: SocketAddr = "203.0.113.91:5000".parse()?;

    for attempt in 1..=3 {
        assert_ne!(
            call(&app, ip, uuid).await,
            StatusCode::TOO_MANY_REQUESTS,
            "attempt {attempt} is inside the budget of 3"
        );
    }

    assert_eq!(
        call(&app, ip, uuid).await,
        StatusCode::TOO_MANY_REQUESTS,
        "the 4th request exceeds a budget of 3"
    );

    Ok(())
}

#[actix_web::test]
#[serial]
async fn test_the_429_carries_retry_after_and_a_message() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;
    let creator = r_data_core_test_support::create_test_admin_user(&pool.pool).await?;
    let uuid = create_provider_workflow(&pool.pool, creator, provider_config(Some(limited(1, 60))))
        .await?;
    let ip: SocketAddr = "203.0.113.92:5000".parse()?;

    call(&app, ip, uuid).await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{uuid}"))
        .peer_addr(ip)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    let retry_after = resp
        .headers()
        .get("Retry-After")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .expect("a 429 must tell the caller when to come back");
    assert!(
        retry_after > 0,
        "Retry-After must be a positive number of seconds"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    let message = body["error"].as_str().unwrap_or_default();
    assert!(
        message.contains('1') && message.contains("60"),
        "the message should name the configured limit, got: {message}"
    );

    Ok(())
}

/// The counter is keyed per workflow, so exhausting one must not affect another.
#[actix_web::test]
#[serial]
async fn test_one_workflow_does_not_consume_another_workflows_budget() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;
    let creator = r_data_core_test_support::create_test_admin_user(&pool.pool).await?;
    let a = create_provider_workflow(&pool.pool, creator, provider_config(Some(limited(2, 60))))
        .await?;
    let b = create_provider_workflow(&pool.pool, creator, provider_config(Some(limited(2, 60))))
        .await?;
    let ip: SocketAddr = "203.0.113.93:5000".parse()?;

    for _ in 0..5 {
        let _ = call(&app, ip, a).await;
    }
    assert_eq!(call(&app, ip, a).await, StatusCode::TOO_MANY_REQUESTS);

    assert_ne!(
        call(&app, ip, b).await,
        StatusCode::TOO_MANY_REQUESTS,
        "workflow B must have its own budget"
    );

    Ok(())
}

/// The chosen scope is per workflow AND per client. Without this test the IP
/// segment could be dropped from the key and everything else would still pass.
#[actix_web::test]
#[serial]
async fn test_one_client_does_not_consume_another_clients_budget() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;
    let creator = r_data_core_test_support::create_test_admin_user(&pool.pool).await?;
    let uuid = create_provider_workflow(&pool.pool, creator, provider_config(Some(limited(2, 60))))
        .await?;
    let noisy: SocketAddr = "203.0.113.94:5000".parse()?;
    let quiet: SocketAddr = "203.0.113.95:5000".parse()?;

    for _ in 0..5 {
        let _ = call(&app, noisy, uuid).await;
    }
    assert_eq!(call(&app, noisy, uuid).await, StatusCode::TOO_MANY_REQUESTS);

    assert_ne!(
        call(&app, quiet, uuid).await,
        StatusCode::TOO_MANY_REQUESTS,
        "one caller must not be able to deny service to another on the same workflow"
    );

    Ok(())
}

/// `increment` sets its TTL with `EXPIRE NX`, so a key minted under the old
/// window would keep enforcing it. The window is part of the key to avoid that.
///
/// The update goes through the admin API on purpose: that is the path that
/// invalidates the workflow cache, so the endpoint actually sees the new
/// window. Writing straight to the repository would leave the cached config in
/// place and test nothing.
#[actix_web::test]
#[serial]
async fn test_changing_the_window_starts_a_fresh_counter() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator = r_data_core_test_support::create_test_admin_user(&pool.pool).await?;
    let uuid = create_provider_workflow(&pool.pool, creator, provider_config(Some(limited(1, 60))))
        .await?;
    let ip: SocketAddr = "203.0.113.96:5000".parse()?;

    call(&app, ip, uuid).await;
    assert_eq!(
        call(&app, ip, uuid).await,
        StatusCode::TOO_MANY_REQUESTS,
        "the budget of 1 should be spent"
    );

    // Same workflow, shorter window: a different key, so a clean slate.
    let repo = r_data_core_persistence::WorkflowRepository::new(pool.pool.clone());
    let existing = r_data_core_persistence::WorkflowRepositoryTrait::get_by_uuid(&repo, uuid)
        .await?
        .expect("workflow exists");

    let update_req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/workflows/{uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "name": existing.name,
            "description": existing.description,
            "kind": existing.kind.to_string(),
            "enabled": existing.enabled,
            "schedule_cron": existing.schedule_cron,
            "config": provider_config(Some(limited(1, 5))),
            "versioning_disabled": existing.versioning_disabled,
        }))
        .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert!(
        update_resp.status().is_success(),
        "the admin update must succeed, got {}",
        update_resp.status()
    );

    assert_ne!(
        call(&app, ip, uuid).await,
        StatusCode::TOO_MANY_REQUESTS,
        "a changed window must mint a fresh counter, not inherit the old one"
    );

    Ok(())
}

/// The limit is enforced in the shared authentication helper, so the ingest
/// route gets it too. Without this test the POST path could be refactored away
/// from that helper and only the GET tests would notice.
#[actix_web::test]
#[serial]
async fn test_the_ingest_route_is_throttled_too() -> anyhow::Result<()> {
    let (app, pool, _token, _) = setup_app_with_entities().await?;
    let creator = r_data_core_test_support::create_test_admin_user(&pool.pool).await?;
    let uuid = create_consumer_workflow(
        &pool.pool,
        creator,
        provider_config(Some(limited(2, 60))),
        true,
        None,
    )
    .await?;
    let ip: SocketAddr = "203.0.113.97:5000".parse()?;

    let post = |body: &'static str| {
        test::TestRequest::post()
            .uri(&format!("/api/v1/workflows/{uuid}"))
            .peer_addr(ip)
            .set_payload(body)
            .to_request()
    };

    for attempt in 1..=2 {
        assert_ne!(
            test::call_service(&app, post("{}")).await.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "attempt {attempt} is inside the budget of 2"
        );
    }

    assert_eq!(
        test::call_service(&app, post("{}")).await.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "the 3rd POST exceeds a budget of 2"
    );

    Ok(())
}
