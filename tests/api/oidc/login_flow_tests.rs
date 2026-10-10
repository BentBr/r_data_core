#![allow(clippy::expect_used, clippy::unwrap_used)]
// The Actix test service is Rc-based, so futures holding it are not Send.
#![allow(clippy::future_not_send)]

//! The browser sign-in endpoints, and what they refuse.
//!
//! These stop short of a completed sign-in — that needs a database, and the
//! interesting behaviour here does not. What is asserted is the part an
//! attacker interacts with: that `start` binds the attempt to the browser
//! that began it, and that `callback` refuses anything not so bound.

use actix_web::{http::StatusCode, test, web, App};

use super::harness::{oidc_routes, state, Idp};

/// The `Set-Cookie` value for the state-binding cookie, if there is one.
fn state_cookie(response: &actix_web::dev::ServiceResponse) -> Option<String> {
    response
        .headers()
        .get_all(actix_web::http::header::SET_COOKIE)
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with("rdc_sso_state="))
        .map(str::to_string)
}

/// The cookie's value, i.e. the state it binds to.
fn bound_state(response: &actix_web::dev::ServiceResponse) -> String {
    state_cookie(response)
        .expect("start must set the binding cookie")
        .trim_start_matches("rdc_sso_state=")
        .split(';')
        .next()
        .expect("a value")
        .to_string()
}

macro_rules! app_with {
    ($idp:expr) => {
        test::init_service(
            App::new()
                .app_data(web::Data::new(state($idp)))
                .configure(oidc_routes),
        )
        .await
    };
}

// ── starting ────────────────────────────────────────────────────────────────

#[actix_web::test]
async fn start_redirects_to_the_provider_and_binds_the_attempt_to_this_browser() {
    let idp = Idp::start().await;
    let app = app_with!(Some(&idp));

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/api/v1/auth/oidc/start")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FOUND);

    let location = response
        .headers()
        .get(actix_web::http::header::LOCATION)
        .expect("a redirect")
        .to_str()
        .expect("utf8");
    assert!(location.contains("code_challenge_method=S256"));
    assert!(location.contains("state="));

    let cookie = state_cookie(&response).expect("the binding cookie");
    assert!(
        cookie.contains("HttpOnly"),
        "script must not be able to read the binding value: {cookie}"
    );
    assert!(
        cookie.contains("SameSite=Lax"),
        "Strict would withhold the cookie on the provider's own callback: {cookie}"
    );
    assert!(
        cookie.contains("Secure"),
        "the harness configures an https redirect URI, so the cookie must be Secure \
         regardless of the scheme the test request arrived on: {cookie}"
    );
    assert!(
        location.contains(&bound_state(&response)),
        "the cookie must bind the very state the provider is being sent"
    );
}

#[actix_web::test]
async fn a_spoofed_forwarded_proto_cannot_drop_the_secure_flag() {
    // Actix reads X-Forwarded-Proto from any peer without checking that it is
    // a trusted proxy. Deriving `Secure` from the connection would therefore
    // let a caller have the cookie issued without it.
    let idp = Idp::start().await;
    let app = app_with!(Some(&idp));

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/api/v1/auth/oidc/start")
            .insert_header(("X-Forwarded-Proto", "http"))
            .to_request(),
    )
    .await;

    let cookie = state_cookie(&response).expect("the binding cookie");
    assert!(
        cookie.contains("Secure"),
        "a request header must not decide whether the cookie is Secure: {cookie}"
    );
}

#[actix_web::test]
async fn start_is_absent_when_the_browser_flow_is_not_configured() {
    let app = app_with!(None);

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/api/v1/auth/oidc/start")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ── the callback's refusals ─────────────────────────────────────────────────

#[actix_web::test]
async fn a_callback_with_no_binding_cookie_is_refused() {
    let idp = Idp::start().await;
    let app = app_with!(Some(&idp));

    // A state this browser never started. This is what login CSRF looks like:
    // the attacker has a valid state from their own sign-in and induces the
    // victim's browser to complete it, which would log the victim into the
    // attacker's account.
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/api/v1/auth/oidc/callback?code=stolen&state=someone-elses")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get(actix_web::http::header::LOCATION)
        .expect("a redirect")
        .to_str()
        .expect("utf8");
    assert!(
        location.contains("sso_error=unbound_state"),
        "expected a refusal, landed at {location}"
    );
}

#[actix_web::test]
async fn a_callback_whose_cookie_names_a_different_state_is_refused() {
    let idp = Idp::start().await;
    let app = app_with!(Some(&idp));

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/api/v1/auth/oidc/callback?code=c&state=state-from-the-attacker")
            .cookie(
                actix_web::cookie::Cookie::build("rdc_sso_state", "the-state-this-browser-started")
                    .finish(),
            )
            .to_request(),
    )
    .await;

    let location = response
        .headers()
        .get(actix_web::http::header::LOCATION)
        .expect("a redirect")
        .to_str()
        .expect("utf8");
    assert!(
        location.contains("sso_error=unbound_state"),
        "a mismatched cookie must not satisfy the binding, landed at {location}"
    );
}

#[actix_web::test]
async fn a_matching_cookie_gets_past_the_binding_check() {
    let idp = Idp::start().await;
    let app = app_with!(Some(&idp));

    // The guard must be narrow: it has to let a genuine sign-in through. This
    // one fails later, at the provider, because the state was never started
    // — what matters is that the failure is no longer `unbound_state`.
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/api/v1/auth/oidc/callback?code=c&state=matching")
            .cookie(actix_web::cookie::Cookie::build("rdc_sso_state", "matching").finish())
            .to_request(),
    )
    .await;

    let location = response
        .headers()
        .get(actix_web::http::header::LOCATION)
        .expect("a redirect")
        .to_str()
        .expect("utf8");
    assert!(
        !location.contains("unbound_state"),
        "a browser completing its own sign-in must get past the binding check"
    );
}

#[actix_web::test]
async fn a_provider_refusal_does_not_become_a_server_error() {
    let idp = Idp::start().await;
    let app = app_with!(Some(&idp));

    // Someone clicked "cancel" on the consent screen.
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/api/v1/auth/oidc/callback?error=access_denied")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get(actix_web::http::header::LOCATION)
        .expect("a redirect")
        .to_str()
        .expect("utf8");
    assert!(location.contains("sso_error=provider_refused"));
}

#[actix_web::test]
async fn a_finished_attempt_clears_its_binding_cookie() {
    let idp = Idp::start().await;
    let app = app_with!(Some(&idp));

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/admin/api/v1/auth/oidc/callback?code=c&state=whatever")
            .to_request(),
    )
    .await;

    let cookie = state_cookie(&response).expect("the cookie should be cleared, not left alone");
    assert!(
        cookie.contains("Max-Age=0") || cookie.contains("rdc_sso_state=;"),
        "a finished attempt must not leave a binding behind: {cookie}"
    );
}
