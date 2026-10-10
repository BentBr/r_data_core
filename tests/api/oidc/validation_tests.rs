#![allow(clippy::expect_used, clippy::unwrap_used)]
// The Actix test service is Rc-based, so futures holding it are not Send.
#![allow(clippy::future_not_send)]

//! What the OIDC arm accepts, what it refuses, and what it leaves alone.
//!
//! Weighted towards refusals on purpose: accepting a good token is one path,
//! and there are a dozen ways to be handed a bad one. The two pass-through
//! tests at the end are the regression guards that matter most — adding a
//! third way to authenticate must not disturb the two that already exist.

use actix_web::{http::StatusCode, test, web, App};
use serde_json::json;

use r_data_core_core::admin_jwt::{generate_jwt, ADMIN_JWT_ISSUER};

use super::harness::{routes, state, Idp, JWT_SECRET};

/// Call the protected route, optionally bearing a token.
async fn call(idp: Option<&Idp>, bearer: Option<&str>) -> StatusCode {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state(idp)))
            .configure(routes),
    )
    .await;

    let mut req = test::TestRequest::get().uri("/whoami");
    if let Some(token) = bearer {
        req = req.insert_header(("Authorization", format!("Bearer {token}")));
    }

    // `try_call_service` rather than `call_service`: middleware refusals come
    // back as a returned error, which a real server renders into a response
    // and the strict helper panics on. The status is what a client sees
    // either way, and that is what these tests are about.
    match test::try_call_service(&app, req.to_request()).await {
        Ok(response) => response.status(),
        Err(error) => error.as_response_error().status_code(),
    }
}

// ── refusals ────────────────────────────────────────────────────────────────

#[actix_web::test]
async fn an_expired_token_is_rejected() {
    let idp = Idp::start().await;
    let expired = idp.token(&json!({ "exp": 1_000_000_000, "iat": 999_999_000 }));

    assert_eq!(
        call(Some(&idp), Some(&expired)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[actix_web::test]
async fn a_token_for_another_audience_is_rejected() {
    let idp = Idp::start().await;
    // Minted for a different service. Accepting it would mean any token from
    // a shared identity provider worked here.
    let wrong = idp.token(&json!({ "aud": "some-other-service" }));

    assert_eq!(
        call(Some(&idp), Some(&wrong)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[actix_web::test]
async fn a_token_with_a_tampered_payload_is_rejected() {
    let idp = Idp::start().await;
    let valid = idp.token(&json!({}));

    // Re-encode the payload with an extra group. The signature no longer
    // covers it, which is the entire point of checking one.
    let mut parts: Vec<String> = valid.split('.').map(str::to_string).collect();
    parts[1] = {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine as _;
        let raw = URL_SAFE_NO_PAD.decode(&parts[1]).expect("payload");
        let mut claims: serde_json::Value = serde_json::from_slice(&raw).expect("json");
        claims["groups"] = json!(["rdc-admins", "rdc-ops"]);
        URL_SAFE_NO_PAD.encode(claims.to_string())
    };
    let forged = parts.join(".");

    assert_eq!(
        call(Some(&idp), Some(&forged)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[actix_web::test]
async fn a_token_with_no_expiry_is_rejected() {
    let idp = Idp::start().await;
    // A token that never expires is a permanent credential.
    let forever = idp.token(&json!({ "exp": null }));

    assert_eq!(
        call(Some(&idp), Some(&forever)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[actix_web::test]
async fn a_token_naming_an_unknown_key_is_rejected_after_one_refresh() {
    let idp = Idp::start().await;
    let valid = idp.token(&json!({}));

    // Swap in a key id the provider does not publish. The runtime re-fetches
    // once — a real rotation must be followed — and then gives up.
    let mut parts: Vec<String> = valid.split('.').map(str::to_string).collect();
    parts[0] = {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine as _;
        URL_SAFE_NO_PAD.encode(json!({ "alg": "RS256", "kid": "never-published" }).to_string())
    };
    let unknown_kid = parts.join(".");

    assert_eq!(
        call(Some(&idp), Some(&unknown_kid)).await,
        StatusCode::UNAUTHORIZED
    );
}

#[actix_web::test]
async fn a_valid_token_whose_lookup_fails_reports_unavailable_not_forbidden() {
    let idp = Idp::start().await;
    let valid = idp.token(&json!({}));

    // The token is genuine; the database behind the resolution is not
    // reachable. An operator seeing 403 here would go looking at role
    // mappings, which is the wrong place entirely.
    assert_eq!(
        call(Some(&idp), Some(&valid)).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

// ── pass-through ────────────────────────────────────────────────────────────

#[actix_web::test]
async fn a_token_from_another_issuer_is_left_for_the_other_auth_arms() {
    let idp = Idp::start().await;
    let other = Idp::start().await;
    // Signed by a provider this instance does not trust. The OIDC arm must
    // not claim it: it belongs to whichever arm recognises that issuer, and
    // here that is nobody, so the route's own guard answers.
    let foreign = other.token(&json!({}));

    assert_eq!(
        call(Some(&idp), Some(&foreign)).await,
        StatusCode::UNAUTHORIZED,
        "the route's own guard should answer, not the OIDC arm"
    );
}

#[actix_web::test]
async fn local_jwt_auth_still_works_with_oidc_enabled() {
    let idp = Idp::start().await;
    let local = local_token();

    assert_eq!(
        call(Some(&idp), Some(&local)).await,
        StatusCode::OK,
        "a local RDataCore JWT must not be run through OIDC validation"
    );
}

#[actix_web::test]
async fn local_jwt_auth_still_works_with_oidc_disabled() {
    assert_eq!(call(None, Some(&local_token())).await, StatusCode::OK);
}

#[actix_web::test]
async fn a_request_with_no_token_is_unaffected() {
    let idp = Idp::start().await;
    assert_eq!(call(Some(&idp), None).await, StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn garbage_in_the_authorization_header_does_not_reach_the_oidc_arm() {
    let idp = Idp::start().await;
    for junk in ["", "not-a-token", "a.b", "a.!!!.c"] {
        assert_eq!(
            call(Some(&idp), Some(junk)).await,
            StatusCode::UNAUTHORIZED,
            "{junk:?} names no issuer, so it is not the OIDC arm's to reject"
        );
    }
}

/// A local `RDataCore` JWT, the kind `login` issues.
fn local_token() -> String {
    use r_data_core_core::admin_user::UserStatus;
    use r_data_core_core::config::ApiConfig;
    use r_data_core_core::domain::AbstractRDataEntity;
    use uuid::Uuid;

    let user = r_data_core_core::admin_user::AdminUser {
        base: AbstractRDataEntity::new("/admin/users".to_string()),
        username: "local-user".to_string(),
        email: "local@example.com".to_string(),
        password_hash: String::new(),
        full_name: "Local User".to_string(),
        status: UserStatus::Active,
        last_login: None,
        failed_login_attempts: 0,
        locked_until: None,
        super_admin: false,
        is_sso_provisioned: false,
        uuid: Uuid::now_v7(),
        first_name: None,
        last_name: None,
        is_active: true,
        is_admin: true,
        created_at: time::OffsetDateTime::now_utc(),
        updated_at: time::OffsetDateTime::now_utc(),
    };

    let config = ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 8888,
        use_tls: false,
        jwt_secret: JWT_SECRET.to_string(),
        jwt_expiration: 3600,
        enable_docs: false,
        cors_origins: vec![],
        check_default_admin_password: false,
    };

    let token = generate_jwt(&user, &config, 3600, &[]).expect("sign a local token");
    assert_eq!(
        r_data_core_core::admin_jwt::verify_jwt(&token, JWT_SECRET)
            .expect("verify")
            .iss,
        ADMIN_JWT_ISSUER
    );
    token
}
