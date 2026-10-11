#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State bookkeeping: a state is usable exactly once, and `return_to` is
//! validated on the way in so a hostile value is never stored.

use super::*;

#[tokio::test]
async fn a_state_we_never_issued_is_refused() {
    let server = provider().await;
    let outcome = flow(&server, &[])
        .complete("never-issued", "some-code", 1800)
        .await;

    assert!(
        matches!(outcome, Err(FlowError::UnknownState)),
        "without this check the callback is a CSRF sink"
    );
}

#[tokio::test]
async fn a_state_cannot_be_claimed_twice() {
    let server = provider().await;
    let flow = flow(&server, &[]);
    let redirect = flow.start(None).await.expect("start");

    // The first attempt gets past the claim and fails later, at the provider
    // — there is no real code to exchange. What matters is that it is *not*
    // refused as an unknown state.
    let first = flow.complete(&redirect.state, "code", 1800).await;
    assert!(
        !matches!(first, Err(FlowError::UnknownState)),
        "the first use of a state must be accepted"
    );

    assert!(
        matches!(
            flow.complete(&redirect.state, "code", 1800).await,
            Err(FlowError::UnknownState)
        ),
        "a state must be single-use, or an intercepted callback can be replayed"
    );
}

#[tokio::test]
async fn a_return_to_leaving_this_origin_is_discarded() {
    let server = provider().await;
    let flow = flow(&server, &[]);

    for escape in [
        "https://evil.example.com/steal",
        "//evil.example.com",
        "javascript:alert(1)",
        "not-a-path",
        // Browsers normalise a backslash to a slash, so each of these
        // resolves to another origin despite starting with a single '/'.
        // The callback appends the session tokens to whatever this returns,
        // so a miss here hands them to whoever chose the destination.
        "/\\evil.example.com",
        "/\\/evil.example.com",
        "/\\\\evil.example.com",
        // Tabs and newlines are stripped before the URL is resolved.
        "/\t/evil.example.com",
        "/\n/evil.example.com",
        "/\r\n//evil.example.com",
    ] {
        assert_eq!(
            flow.landing_path(Some(escape)),
            "/dashboard",
            "{escape:?} must not be somewhere this login endpoint sends a browser"
        );
    }
}

#[tokio::test]
async fn a_local_return_to_is_honoured() {
    let server = provider().await;
    assert_eq!(
        flow(&server, &[]).landing_path(Some("/workflows/42")),
        "/workflows/42"
    );
}

#[tokio::test]
async fn the_configured_landing_path_is_used_when_none_is_asked_for() {
    let server = provider().await;
    assert_eq!(
        flow(&server, &[("RDC_OIDC_POST_LOGIN_PATH", "/admin/dashboard")]).landing_path(None),
        "/admin/dashboard"
    );
}

#[tokio::test]
async fn a_hostile_return_to_is_not_even_stored() {
    let server = provider().await;
    let flow = flow(&server, &[]);
    let redirect = flow
        .start(Some("https://evil.example.com/steal"))
        .await
        .expect("start");

    let pending: PendingLogin = flow
        .cache
        .get(&format!("{STATE_PREFIX}{}", redirect.state))
        .await
        .expect("cache")
        .expect("stored");

    assert!(
        pending.return_to.is_none(),
        "validating only on the way out would leave a hostile value one bug from being used"
    );
}
