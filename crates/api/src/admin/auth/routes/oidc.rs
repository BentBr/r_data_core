#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix handlers take HttpRequest which is !Send

//! Signing in through an external identity provider.
//!
//! Three endpoints, one shared tail. `start` and `callback` are the browser
//! flow; `exchange` is the same operation for a machine caller that already
//! holds a provider token. All three resolve the identity through
//! `OidcServices`, so the account-status check and the role mapping cannot
//! differ between them.
//!
//! **Why the callback hands tokens back in a URL fragment.** The browser must
//! end up holding the same access and refresh tokens a password login
//! produces, because that is what lets refresh, logout and revoke-all work
//! unchanged. A fragment is the one part of a URL that browsers never send to
//! a server, so it does not land in access logs, proxies or `Referer`
//! headers the way a query string would.

use actix_web::{get, http::header, post, web, HttpResponse, Responder};
use serde::Deserialize;
use std::sync::Arc;

use crate::admin::auth::rate_limit;
use crate::admin::auth::routes::helpers::issue_session;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::response::ApiResponse;
use r_data_core_core::admin_jwt::ACCESS_TOKEN_EXPIRY_SECONDS;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_services::{FlowError, OidcAuthError};

/// Query accepted by `start`.
#[derive(Debug, Deserialize)]
pub struct StartQuery {
    /// Where to land afterwards. Validated as a path inside this application;
    /// anything else is discarded rather than followed.
    pub return_to: Option<String>,
}

/// Query the provider sends back.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    /// Set when the provider itself refused, e.g. the user cancelled.
    pub error: Option<String>,
}

/// Body accepted by `exchange`.
#[derive(Debug, Deserialize)]
pub struct ExchangeRequest {
    pub token: String,
}

/// Begin a single-sign-on login.
#[utoipa::path(
    get,
    path = "/admin/api/v1/auth/oidc/start",
    tag = "admin-auth",
    responses(
        (status = 302, description = "Redirect to the identity provider"),
        (status = 404, description = "Single sign-on is not configured"),
        (status = 503, description = "The identity provider is unreachable")
    ),
    security()
)]
#[get("/auth/oidc/start")]
pub async fn oidc_start(
    data: web::Data<ApiStateWrapper>,
    query: web::Query<StartQuery>,
) -> impl Responder {
    let Some(flow) = configured_flow(&data) else {
        return ApiResponse::<()>::not_found("Single sign-on is not configured");
    };

    match flow.start(query.return_to.as_deref()).await {
        Ok(redirect) => HttpResponse::Found()
            .insert_header((header::LOCATION, redirect.url))
            // A sign-in redirect is specific to this attempt and must never be
            // reused from a cache.
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .finish(),
        Err(e) => {
            log::error!("Could not start the single-sign-on flow: {e}");
            ApiResponse::<()>::service_unavailable("Single sign-on is temporarily unavailable")
        }
    }
}

/// Complete a single-sign-on login.
#[utoipa::path(
    get,
    path = "/admin/api/v1/auth/oidc/callback",
    tag = "admin-auth",
    responses(
        (status = 302, description = "Redirect into the admin interface with a session"),
        (status = 404, description = "Single sign-on is not configured")
    ),
    security()
)]
#[get("/auth/oidc/callback")]
pub async fn oidc_callback(
    data: web::Data<ApiStateWrapper>,
    query: web::Query<CallbackQuery>,
) -> impl Responder {
    let Some(flow) = configured_flow(&data) else {
        return ApiResponse::<()>::not_found("Single sign-on is not configured");
    };

    // The provider refused before we were involved — a cancelled consent
    // screen, usually. Not an error worth a 500.
    if let Some(reason) = &query.error {
        log::info!("The identity provider refused a sign-in: {reason}");
        return failed_login(&flow.landing_path(None), "provider_refused");
    }

    let (Some(code), Some(state)) = (&query.code, &query.state) else {
        return failed_login(&flow.landing_path(None), "incomplete_callback");
    };

    let (claims, return_to) = match flow
        .complete(state, code, ACCESS_TOKEN_EXPIRY_SECONDS)
        .await
    {
        Ok(resolved) => resolved,
        Err(e) => {
            log::warn!("Single-sign-on callback failed: {e}");
            return failed_login(&flow.landing_path(None), reason_for(&e));
        }
    };

    let landing = flow.landing_path(return_to.as_deref());

    // The identity is good. Turn it into the same session a password login
    // produces, so nothing downstream has to know how it began.
    let repo = AdminUserRepository::new(Arc::new(data.db_pool().clone()));
    let Some(uuid) = resolved_user_uuid(&data, &claims.issuer, &claims.subject).await else {
        return failed_login(&landing, "unresolved_identity");
    };
    let Ok(Some(mut user)) = repo.find_by_uuid(&uuid).await else {
        return failed_login(&landing, "unresolved_identity");
    };

    let Some(session) = issue_session(&mut user, &repo, &data).await else {
        return failed_login(&landing, "session_failed");
    };

    let fragment = format!(
        "access_token={}&refresh_token={}&access_expires_at={}&refresh_expires_at={}",
        urlencode(&session.tokens.access_token),
        urlencode(&session.tokens.refresh_token),
        session.tokens.access_expires_at.unix_timestamp(),
        session.tokens.refresh_expires_at.unix_timestamp(),
    );

    HttpResponse::Found()
        .insert_header((header::LOCATION, format!("{landing}#{fragment}")))
        // Tokens travel in this response. It must not be stored anywhere.
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .finish()
}

/// Trade a provider token for a local access token.
///
/// For machine callers — the MCP server, principally. It presents the token
/// it validated for its caller and receives one issued by `RDataCore` for
/// that same person. Forwarding the original token onward instead is the
/// pattern the MCP authorization specification names and disallows.
#[utoipa::path(
    post,
    path = "/admin/api/v1/auth/oidc/exchange",
    tag = "admin-auth",
    request_body = String,
    responses(
        (status = 200, description = "A local access token for the presented identity"),
        (status = 401, description = "The presented token was not accepted"),
        (status = 403, description = "The identity may not sign in"),
        (status = 404, description = "Single sign-on is not configured"),
        (status = 429, description = "Too many exchange attempts"),
        (status = 503, description = "The identity provider is unreachable")
    ),
    security()
)]
#[post("/auth/oidc/exchange")]
pub async fn oidc_exchange(
    req: actix_web::HttpRequest,
    data: web::Data<ApiStateWrapper>,
    body: Option<web::Json<ExchangeRequest>>,
) -> impl Responder {
    let Some(oidc) = data.oidc().cloned() else {
        return ApiResponse::<()>::not_found("Single sign-on is not configured");
    };
    let Some(body) = body else {
        return ApiResponse::bad_request("Missing or invalid JSON body");
    };

    // Every call costs a provider round trip and a database lookup, and it is
    // reachable without a local credential. Throttle it like login.
    let rl_key = rate_limit::rate_limit_key(&req, rate_limit::Bucket::Login);
    if rate_limit::is_over_limit(data.cache_manager(), &rl_key).await {
        log::warn!("Token-exchange rate limit hit for key {rl_key}");
        return ApiResponse::too_many_requests("Too many exchange attempts, try again later");
    }

    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let claims = match oidc.runtime().verify(&body.token, now).await {
        Ok(claims) => claims,
        Err(e) => {
            rate_limit::record_failure(data.cache_manager(), &rl_key).await;
            return refusal(&e);
        }
    };

    // Resolve through the shared tail, so an unmapped or deactivated identity
    // is refused here on exactly the terms the bearer path uses.
    if let Err(e) = oidc
        .runtime()
        .session_claims(&claims, ACCESS_TOKEN_EXPIRY_SECONDS)
        .await
    {
        return refusal(&e);
    }

    let repo = AdminUserRepository::new(Arc::new(data.db_pool().clone()));
    let Some(uuid) = resolved_user_uuid(&data, &claims.issuer, &claims.subject).await else {
        return ApiResponse::internal_error("Authentication failed");
    };
    let Ok(Some(user)) = repo.find_by_uuid(&uuid).await else {
        return ApiResponse::internal_error("Authentication failed");
    };

    let roles = crate::admin::auth::routes::helpers::load_user_roles(&user, &data, &repo).await;
    let access_token = match r_data_core_core::admin_jwt::generate_access_token(
        &user,
        data.api_config(),
        &roles,
    ) {
        Ok(token) => token,
        Err(e) => {
            log::error!("Failed to generate an exchanged access token: {e:?}");
            return ApiResponse::internal_error("Authentication failed");
        }
    };

    // No refresh token. A machine caller re-exchanges instead, which keeps
    // the revocation window as short as the access token and avoids a
    // long-lived credential living in the MCP server.
    ApiResponse::ok(serde_json::json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": ACCESS_TOKEN_EXPIRY_SECONDS,
    }))
}

/// The flow, if single sign-on is configured for browser logins.
fn configured_flow(data: &ApiStateWrapper) -> Option<&r_data_core_services::OidcFlow> {
    data.oidc()
        .map(|oidc| oidc.flow())
        .filter(|flow| flow.is_configured())
}

/// The `RDataCore` user an identity resolved to.
async fn resolved_user_uuid(
    data: &ApiStateWrapper,
    issuer: &str,
    subject: &str,
) -> Option<uuid::Uuid> {
    use r_data_core_persistence::IdentityRepositoryTrait as _;

    // Resolution has already run, so the link exists; this only reads it back.
    let identities = r_data_core_persistence::IdentityRepository::new(data.db_pool().clone());
    match identities.find_user_by_identity(issuer, subject).await {
        Ok(found) => found,
        Err(e) => {
            log::error!("Could not read back the resolved identity: {e:?}");
            None
        }
    }
}

/// Send the browser back to the sign-in page with a reason it can show.
///
/// A code rather than a message: the detail is in the server log, and an
/// error string reflected into a page is one more thing to get wrong.
fn failed_login(landing: &str, reason: &str) -> HttpResponse {
    HttpResponse::Found()
        .insert_header((header::LOCATION, format!("{landing}#sso_error={reason}")))
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .finish()
}

/// A stable, non-revealing code for a failed sign-in.
const fn reason_for(error: &FlowError) -> &'static str {
    match error {
        FlowError::UnknownState => "expired_or_replayed",
        FlowError::Auth(OidcAuthError::Denied(_)) => "not_permitted",
        FlowError::Auth(OidcAuthError::Rejected(_)) => "invalid_token",
        _ => "provider_error",
    }
}

/// Map an authentication failure onto a response.
///
/// The 403/503 split matters: a locked account and an unreachable provider
/// are different operational problems, and collapsing them sends whoever is
/// on call to the wrong place.
fn refusal(error: &OidcAuthError) -> HttpResponse {
    log::warn!("Token exchange refused: {error}");
    match error {
        OidcAuthError::Denied(_) => ApiResponse::<()>::forbidden("Not permitted to sign in"),
        OidcAuthError::Rejected(_) => ApiResponse::<()>::unauthorized("Invalid token"),
        OidcAuthError::Keys(_) | OidcAuthError::Unavailable(_) => {
            ApiResponse::<()>::service_unavailable("Authentication is temporarily unavailable")
        }
    }
}

/// Percent-encode a value for a URL fragment.
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}
