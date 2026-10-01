#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix handlers take HttpRequest which is !Send

//! Opt-in, per-workflow request throttling for the public API.
//!
//! Deliberately not a global limiter: workflows are independently owned and
//! independently consumed, so one workflow's traffic must never spend another
//! workflow's budget.

use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;
use uuid::Uuid;

use r_data_core_core::config::SecurityConfig;
use r_data_core_workflow::data::{Workflow, WorkflowRateLimit};

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::client_ip::client_ip_key;

/// Counter key for one workflow, one window and one caller.
///
/// The workflow UUID keeps budgets independent per workflow. The client segment
/// stops one abusive caller exhausting the legitimate consumer's budget — drop
/// it to turn this into a per-workflow quota instead.
///
/// The window is in the key because [`crate::admin::auth::rate_limit`]'s
/// counter sets its TTL with `EXPIRE NX`: an existing key keeps its original
/// window forever. Without this segment, shortening a window from 60 minutes to
/// 5 would leave callers enforced against the old one until it lapsed. A new
/// window mints a new key, which self-heals with no invalidation and no
/// scan-and-delete. `max_requests` is deliberately *not* in the key: it is
/// compared live, so changing it takes effect on the very next request.
fn rate_limit_key_for_workflow(req: &HttpRequest, workflow: &Uuid, window_minutes: u32) -> String {
    let ip = client_ip_key(req, &SecurityConfig::global().trusted_proxies);
    format!("workflow_rl:{workflow}:{window_minutes}:{ip}")
}

/// Apply the workflow's configured limit, if it has one.
///
/// A workflow with no limit, a disabled limit, or a malformed one is not
/// throttled at all.
pub(super) async fn enforce_workflow_rate_limit(
    req: &HttpRequest,
    workflow: &Workflow,
    state: &web::Data<ApiStateWrapper>,
) -> Result<(), HttpResponse> {
    let Some(limit) = WorkflowRateLimit::from_config(&workflow.config) else {
        return Ok(());
    };

    let key = rate_limit_key_for_workflow(req, &workflow.uuid, limit.window_minutes);

    // Increment first, then compare. Reading and then writing would let two
    // parallel callers both observe a stale count and both pass; the atomic
    // increment means the Nth request sees exactly N.
    let count = state
        .cache_manager()
        .increment(&key, limit.window_secs())
        .await
        .unwrap_or(0);

    if count > limit.max_requests {
        log::warn!(
            "Workflow {} rate limit hit ({} > {})",
            workflow.uuid,
            count,
            limit.max_requests
        );

        // Retry-After is the FULL window, not the time remaining in it.
        // `CacheManager` exposes no TTL read, and over-estimating is the safe
        // direction: a client that waits this long is certainly clear again.
        return Err(HttpResponse::TooManyRequests()
            .insert_header(("Retry-After", limit.window_secs().to_string()))
            .json(json!({
                "error": format!(
                    "Rate limit exceeded: this workflow allows {} requests per {} minutes",
                    limit.max_requests, limit.window_minutes
                )
            })));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::rate_limit_key_for_workflow;
    use actix_web::test::TestRequest;
    use uuid::Uuid;

    #[test]
    fn the_key_is_scoped_to_the_workflow() {
        let req = TestRequest::default()
            .peer_addr(
                "203.0.113.5:5000"
                    .parse()
                    .unwrap_or_else(|_| unreachable!()),
            )
            .to_http_request();

        assert_ne!(
            rate_limit_key_for_workflow(&req, &Uuid::now_v7(), 60),
            rate_limit_key_for_workflow(&req, &Uuid::now_v7(), 60),
            "two workflows must never share a budget"
        );
    }

    #[test]
    fn the_key_is_scoped_to_the_client() {
        let workflow = Uuid::now_v7();
        let a = TestRequest::default()
            .peer_addr(
                "203.0.113.5:5000"
                    .parse()
                    .unwrap_or_else(|_| unreachable!()),
            )
            .to_http_request();
        let b = TestRequest::default()
            .peer_addr(
                "203.0.113.6:5000"
                    .parse()
                    .unwrap_or_else(|_| unreachable!()),
            )
            .to_http_request();

        assert_ne!(
            rate_limit_key_for_workflow(&a, &workflow, 60),
            rate_limit_key_for_workflow(&b, &workflow, 60),
            "one caller must not be able to exhaust another's budget"
        );
    }

    /// Regression guard for the `EXPIRE NX` behaviour: a changed window has to
    /// produce a different key, or the old window keeps being enforced.
    #[test]
    fn changing_the_window_changes_the_key() {
        let workflow = Uuid::now_v7();
        let req = TestRequest::default()
            .peer_addr(
                "203.0.113.5:5000"
                    .parse()
                    .unwrap_or_else(|_| unreachable!()),
            )
            .to_http_request();

        assert_ne!(
            rate_limit_key_for_workflow(&req, &workflow, 60),
            rate_limit_key_for_workflow(&req, &workflow, 5),
        );
    }

    #[test]
    fn the_key_is_namespaced_away_from_the_admin_limiter() {
        let req = TestRequest::default()
            .peer_addr(
                "203.0.113.5:5000"
                    .parse()
                    .unwrap_or_else(|_| unreachable!()),
            )
            .to_http_request();

        assert!(rate_limit_key_for_workflow(&req, &Uuid::now_v7(), 60).starts_with("workflow_rl:"));
    }
}
