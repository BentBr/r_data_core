#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Exchanging the code: provider refusals and malformed responses are reported
//! without leaking the code, and the client secret is sent only when configured.

use super::*;

#[tokio::test]
async fn a_provider_that_refuses_the_exchange_is_reported_without_the_code() {
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        // Providers routinely echo the submitted parameters into an error
        // body. None of that belongs in this server's logs.
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string("invalid_grant: an-authorization-code was already used"),
        )
        .mount(&server)
        .await;

    let error = round_trip(&flow(&server, &[]))
        .await
        .expect_err("the exchange should fail");
    assert!(
        matches!(error, FlowError::Provider(_)),
        "expected a provider error, got {error:?}"
    );
    assert!(
        !error.to_string().contains("an-authorization-code"),
        "the authorization code must not travel into the error: {error}"
    );
}

#[tokio::test]
async fn a_token_response_without_an_id_token_is_refused() {
    // Some providers answer with an opaque access token and nothing else.
    // There is no identity in that, and guessing is not an option.
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "opaque-and-useless-here",
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    let outcome = round_trip(&flow(&server, &[])).await;

    assert!(
        matches!(outcome, Err(FlowError::NoIdToken)),
        "expected NoIdToken, got {outcome:?}"
    );
}

#[tokio::test]
async fn an_unreadable_token_response_is_a_provider_error() {
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>not json</html>"))
        .mount(&server)
        .await;

    let outcome = round_trip(&flow(&server, &[])).await;

    assert!(
        matches!(outcome, Err(FlowError::Provider(_))),
        "expected a provider error, got {outcome:?}"
    );
}

#[tokio::test]
async fn a_discovery_document_without_a_token_endpoint_is_refused() {
    let server = provider_without_token_endpoint().await;
    let flow = flow(&server, &[]);

    // `start` works — the authorization endpoint is there — and the failure
    // only appears at the exchange, which is exactly the confusing case worth
    // naming precisely.
    let outcome = round_trip(&flow).await;

    assert!(
        matches!(outcome, Err(FlowError::MissingEndpoint("token_endpoint"))),
        "expected a named missing endpoint, got {outcome:?}"
    );
}

#[tokio::test]
async fn the_client_secret_is_sent_when_one_is_configured() {
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id_token": "x.y.z" })))
        .mount(&server)
        .await;

    let flow = flow(&server, &[("RDC_OIDC_CLIENT_SECRET", "s3cret")]);
    // Fails later at validation — the point is what reached the provider.
    let _ = round_trip(&flow).await;

    let requests = server.received_requests().await.expect("requests");
    let body = requests
        .iter()
        .find(|r| r.url.path() == "/token")
        .map(|r| String::from_utf8_lossy(&r.body).to_string())
        .expect("a token request");

    assert!(body.contains("code_verifier="), "PKCE must be sent: {body}");
    assert!(body.contains("client_secret=s3cret"), "{body}");
    assert!(body.contains("grant_type=authorization_code"), "{body}");
}

#[tokio::test]
async fn no_client_secret_is_sent_for_a_public_client() {
    let server = provider().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id_token": "x.y.z" })))
        .mount(&server)
        .await;

    let _ = round_trip(&flow(&server, &[])).await;

    let requests = server.received_requests().await.expect("requests");
    let body = requests
        .iter()
        .find(|r| r.url.path() == "/token")
        .map(|r| String::from_utf8_lossy(&r.body).to_string())
        .expect("a token request");

    assert!(
        !body.contains("client_secret"),
        "a public client sends none; PKCE is what protects the exchange: {body}"
    );
}
