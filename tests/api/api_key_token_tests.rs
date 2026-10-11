#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::expect_used, clippy::unwrap_used)]
#![allow(clippy::future_not_send)] // Test functions hold impl Service across awaits

//! Trading an API key for an admin access token.
//!
//! The token must be exactly as powerful as the key's owner and no more; the
//! exchange must be the place a revoked, expired or barred-owner key stops
//! working; a JWT must not be able to mint a fresh JWT through it; and a
//! successful mint must leave an audit row that holds no key material.

use actix_web::{http::StatusCode, test};
use serial_test::serial;
use uuid::Uuid;

use r_data_core_core::admin_jwt::{verify_jwt, ACCESS_TOKEN_EXPIRY_SECONDS};
use r_data_core_core::admin_user::ApiKey;
use r_data_core_core::config::security::API_KEY_EXCHANGE_RATE_LIMIT_MAX;

use super::users::common::{get_auth_token, setup_test_app, setup_test_app_with_audit};

const EXCHANGE: &str = "/admin/api/v1/auth/api-key/token";
const KEY_NAME: &str = "exchange test";

/// A non-super-admin owner with one role granting `workflows:read`, and a
/// key for them.
///
/// Returns `(owner uuid, key uuid, plaintext key)`.
async fn key_owner(pool: &sqlx::PgPool) -> (Uuid, Uuid, String) {
    let tag = Uuid::now_v7().simple().to_string();
    let owner: Uuid = sqlx::query_scalar(
        "INSERT INTO admin_users (username, email, password_hash, first_name, last_name, \
         is_active, created_by, super_admin, published) \
         VALUES ($1, $2, 'not-a-usable-hash', 'Key', 'Owner', TRUE, $3, FALSE, TRUE) \
         RETURNING uuid",
    )
    .bind(format!("owner_{tag}"))
    .bind(format!("owner_{tag}@example.test"))
    .bind(Uuid::nil())
    .fetch_one(pool)
    .await
    .expect("owner");

    let role: Uuid = sqlx::query_scalar(
        "INSERT INTO roles (name, description, permissions, created_by) \
         VALUES ($1, 'api-key exchange test', $2, $3) RETURNING uuid",
    )
    .bind(format!("role_{tag}"))
    .bind(serde_json::json!([{
        "resource_type": "Workflows",
        "permission_type": "Read",
        "access_level": "All",
        "resource_uuids": [],
        "constraints": null,
    }]))
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("role");
    sqlx::query("INSERT INTO user_roles (user_uuid, role_uuid) VALUES ($1, $2)")
        .bind(owner)
        .bind(role)
        .execute(pool)
        .await
        .expect("assign");

    let plaintext = format!("rdc_test_{tag}");
    let key: Uuid = sqlx::query_scalar(
        "INSERT INTO api_keys (user_uuid, name, key_hash, is_active, created_by, published) \
         VALUES ($1, $2, $3, TRUE, $1, TRUE) RETURNING uuid",
    )
    .bind(owner)
    .bind(KEY_NAME)
    .bind(ApiKey::hash_api_key(&plaintext).expect("hash"))
    .fetch_one(pool)
    .await
    .expect("key");

    (owner, key, plaintext)
}

fn with_key(key: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri(EXCHANGE)
        .insert_header(("X-API-Key", key.to_string()))
}

async fn exec(pool: &sqlx::PgPool, sql: &str, id: Uuid) {
    sqlx::query(sql)
        .bind(id)
        .execute(pool)
        .await
        .expect("update");
}

#[serial]
#[tokio::test]
async fn a_valid_key_mints_a_token_bounded_by_its_owner() {
    let (app, pool, _) = setup_test_app().await.unwrap();
    let (owner, _, key) = key_owner(&pool.pool).await;

    let resp = test::call_service(&app, with_key(&key).to_request()).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["token_type"], "Bearer");
    assert_eq!(body["data"]["expires_in"], ACCESS_TOKEN_EXPIRY_SECONDS);
    assert!(body["data"].get("refresh_token").is_none());
    let token = body["data"]["access_token"].as_str().unwrap();
    let claims = verify_jwt(token, "test_secret").unwrap();
    assert_eq!(claims.sub, owner.to_string());
    assert!(
        !claims.is_super_admin,
        "a key never outranks a non-admin owner"
    );
    assert_eq!(claims.permissions, vec!["workflows:read".to_string()]);
}

#[serial]
#[tokio::test]
async fn an_unknown_key_is_refused() {
    let (app, _pool, _) = setup_test_app().await.unwrap();

    let resp = test::call_service(&app, with_key("rdc_not_a_key").to_request()).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn a_revoked_key_is_refused() {
    let (app, pool, _) = setup_test_app().await.unwrap();
    let (_, key_uuid, key) = key_owner(&pool.pool).await;
    exec(
        &pool.pool,
        "UPDATE api_keys SET is_active = FALSE WHERE uuid = $1",
        key_uuid,
    )
    .await;

    let resp = test::call_service(&app, with_key(&key).to_request()).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn an_expired_key_is_refused() {
    let (app, pool, _) = setup_test_app().await.unwrap();
    let (_, key_uuid, key) = key_owner(&pool.pool).await;
    exec(
        &pool.pool,
        "UPDATE api_keys SET expires_at = NOW() - INTERVAL '1 minute' WHERE uuid = $1",
        key_uuid,
    )
    .await;

    let resp = test::call_service(&app, with_key(&key).to_request()).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn a_key_whose_owner_is_deactivated_is_refused() {
    let (app, pool, _) = setup_test_app().await.unwrap();
    let (owner, _, key) = key_owner(&pool.pool).await;
    exec(
        &pool.pool,
        "UPDATE admin_users SET is_active = FALSE WHERE uuid = $1",
        owner,
    )
    .await;

    let resp = test::call_service(&app, with_key(&key).to_request()).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"]["access_token"].is_null(), "{body}");
}

#[serial]
#[tokio::test]
async fn a_key_whose_owner_is_locked_is_refused() {
    let (app, pool, _) = setup_test_app().await.unwrap();
    let (owner, _, key) = key_owner(&pool.pool).await;
    exec(
        &pool.pool,
        "UPDATE admin_users SET locked_until = NOW() + INTERVAL '1 hour' WHERE uuid = $1",
        owner,
    )
    .await;

    let resp = test::call_service(&app, with_key(&key).to_request()).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"]["access_token"].is_null(), "{body}");
}

#[serial]
#[tokio::test]
async fn a_jwt_cannot_mint_another_jwt_here() {
    let (app, pool, _) = setup_test_app().await.unwrap();
    let token = get_auth_token(&app, &pool).await;

    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(EXCHANGE)
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request(),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[serial]
#[tokio::test]
async fn minting_is_throttled_at_the_exchange_threshold() {
    // Only valid keys reach the handler (the extractor refuses the rest first),
    // so the bucket bounds minting, not guessing. Full loop: the threshold
    // itself is what is under test.
    let (app, pool, _) = setup_test_app().await.unwrap();
    let (_, _, key) = key_owner(&pool.pool).await;

    for n in 0..API_KEY_EXCHANGE_RATE_LIMIT_MAX {
        let resp = test::call_service(&app, with_key(&key).to_request()).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "mint {n} is within the limit"
        );
    }
    let resp = test::call_service(&app, with_key(&key).to_request()).await;

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[serial]
#[tokio::test]
async fn a_mint_is_audited_without_key_material() {
    let (app, pool, _) = setup_test_app_with_audit(true).await.unwrap();
    let (owner, key_uuid, key) = key_owner(&pool.pool).await;

    let resp = test::call_service(&app, with_key(&key).to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let token = body["data"]["access_token"].as_str().unwrap().to_string();

    let (actor, details): (Option<Uuid>, Option<serde_json::Value>) = sqlx::query_as(
        "SELECT created_by, details FROM system_logs \
         WHERE log_type = 'auth_event' AND resource_uuid = $1",
    )
    .bind(key_uuid)
    .fetch_one(&pool.pool)
    .await
    .expect("exactly one audit row for this key");
    assert_eq!(actor, Some(owner));
    let details = details.expect("details").to_string();
    assert!(details.contains("api_key_exchange"), "{details}");
    assert!(details.contains(KEY_NAME), "{details}");
    assert!(!details.contains(&key), "the key itself must not be logged");
    assert!(!details.contains(&token), "the token must not be logged");
    let key_hash: String = sqlx::query_scalar("SELECT key_hash FROM api_keys WHERE uuid = $1")
        .bind(key_uuid)
        .fetch_one(&pool.pool)
        .await
        .unwrap();
    assert!(!details.contains(&key_hash), "nor its hash");
}
