#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The audit trail of a provider-token exchange.
//!
//! A successful exchange writes one `auth_event` row naming the person as
//! both actor and resource. The row must say what happened and nothing that
//! could be replayed: no token material.
//!
//! Serialised: resource-heavy stacks (full server + MCP + migrated schema).

use serial_test::serial;
use uuid::Uuid;

use crate::mcp::oauth_harness::OauthStack;

/// actor, resource, resource type, details.
type AuditRow = (
    Option<Uuid>,
    Option<Uuid>,
    String,
    Option<serde_json::Value>,
);

#[tokio::test]
#[serial]
async fn a_successful_exchange_writes_exactly_one_audit_row() {
    let stack = OauthStack::start_with_audit("rdc-admins:admin", &[], true).await;
    let token = stack.mint_token("audited", &["rdc-admins"]);

    let response = reqwest::Client::new()
        .post(format!("{}/admin/api/v1/auth/oidc/exchange", stack.rdc_url))
        .json(&serde_json::json!({ "token": token }))
        .send()
        .await
        .expect("a response");
    assert!(response.status().is_success(), "got {}", response.status());
    let body: serde_json::Value = response.json().await.expect("json");
    let access = body["data"]["access_token"]
        .as_str()
        .expect("an access token")
        .to_string();

    let user: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users WHERE email = $1")
        .bind("audited@example.com")
        .fetch_one(stack.pool())
        .await
        .expect("the provisioned user");

    let rows: Vec<AuditRow> =
        sqlx::query_as(
            "SELECT created_by, resource_uuid, resource_type::text, details \
             FROM system_logs WHERE log_type = 'auth_event' AND details->>'action' = 'oidc_exchange'",
        )
        .fetch_all(stack.pool())
        .await
        .expect("audit rows");

    assert_eq!(rows.len(), 1, "exactly one oidc_exchange audit row");
    let (actor, resource, resource_type, details) = rows.into_iter().next().unwrap();
    assert_eq!(actor, Some(user), "the actor is the person");
    assert_eq!(resource, Some(user), "the resource is the person");
    assert_eq!(resource_type, "admin_user");
    let details = details.expect("details");
    assert_eq!(details, serde_json::json!({ "action": "oidc_exchange" }));

    let text = details.to_string();
    assert!(
        !text.contains(&token) && !text.contains(&access),
        "the audit row must carry no token material: {text}"
    );
}
