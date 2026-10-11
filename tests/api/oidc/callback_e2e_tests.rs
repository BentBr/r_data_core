#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The browser sign-in, all the way through, against a real `RDataCore`.
//!
//! `login_flow_tests.rs` stops at the binding check because it has no
//! database. These drive `start` → provider → `callback` with one, and assert
//! what the resulting session can do — which is the part that was wrong.
//!
//! Serialised: the harness sets `RDC_OIDC_*` in the process environment.

use base64::Engine as _;
use serial_test::serial;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::mcp::oauth_harness::OauthStack;

const CALLBACK: &str = "/admin/api/v1/auth/oidc/callback";

/// A client that does not follow redirects: every assertion here is about a
/// `Location` header, and the landing page is not served by this server.
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("a client")
}

/// Begin a sign-in. Returns the state, the nonce sent to the provider, and
/// the binding cookie.
async fn start(stack: &OauthStack) -> (String, String, String) {
    let response = client()
        .get(format!("{}/admin/api/v1/auth/oidc/start", stack.rdc_url))
        .send()
        .await
        .expect("start");
    assert_eq!(
        response.status(),
        302,
        "start must redirect to the provider"
    );

    let location = response.headers()["location"]
        .to_str()
        .expect("utf8")
        .to_string();
    let param = |name: &str| {
        location
            .split('?')
            .nth(1)
            .expect("a query")
            .split('&')
            .find_map(|pair| pair.strip_prefix(&format!("{name}=")))
            .expect(name)
            .to_string()
    };
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with("rdc_sso_state="))
        .expect("the binding cookie")
        .split(';')
        .next()
        .expect("a value")
        .to_string();

    (param("state"), param("nonce"), cookie)
}

/// Complete a sign-in with `id_token` waiting at the provider.
async fn callback(stack: &OauthStack, state: &str, cookie: &str, id_token: &str) -> String {
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id_token": id_token,
        })))
        .mount(stack.idp())
        .await;

    let response = client()
        .get(format!("{}{CALLBACK}?code=c&state={state}", stack.rdc_url))
        .header("Cookie", cookie)
        .send()
        .await
        .expect("callback");
    assert_eq!(response.status(), 302);
    response.headers()["location"]
        .to_str()
        .expect("utf8")
        .to_string()
}

/// The `permissions` claim of the access token in a landing fragment.
fn permissions_in(location: &str) -> Vec<String> {
    let fragment = location.split('#').nth(1).expect("a fragment");
    let access = fragment
        .split('&')
        .find_map(|pair| pair.strip_prefix("access_token="))
        .expect("an access token");
    let payload = access.split('.').nth(1).expect("a payload");
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .expect("base64");
    let claims: serde_json::Value = serde_json::from_slice(&decoded).expect("json");
    claims["permissions"]
        .as_array()
        .expect("permissions")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
#[serial]
async fn a_completed_sign_in_lands_with_the_mapped_permissions() {
    let stack = OauthStack::start("rdc-ops:sso-editor").await;
    stack
        .grant_permission("sso-editor", "Workflows", "Read")
        .await;

    let (state, nonce, cookie) = start(&stack).await;
    let location = callback(
        &stack,
        &state,
        &cookie,
        &stack.mint_id_token("ada", &["rdc-ops"], &nonce),
    )
    .await;

    assert!(
        !location.contains("sso_error"),
        "a genuine sign-in must not be refused: {location}"
    );
    assert_eq!(
        permissions_in(&location),
        vec!["workflows:read".to_string()]
    );
}

#[tokio::test]
#[serial]
async fn an_id_token_minted_for_another_attempt_is_refused() {
    let stack = OauthStack::start("rdc-ops:sso-editor").await;

    let (state, _nonce, cookie) = start(&stack).await;
    let location = callback(
        &stack,
        &state,
        &cookie,
        &stack.mint_id_token("ada", &["rdc-ops"], "a-nonce-from-elsewhere"),
    )
    .await;

    assert!(
        location.contains("sso_error=invalid_token"),
        "a token not bound to this attempt must be refused: {location}"
    );
}
