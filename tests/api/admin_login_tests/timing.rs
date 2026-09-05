//! Username enumeration via response timing.
//!
//! Bodies for "no such user" and "wrong password" are already identical. The
//! remaining signal was work: the miss path used to return before any Argon2
//! verification, making it an order of magnitude faster than a hit. The dummy
//! verification closes that.
//!
//! Timing assertions are inherently noisy, so this asserts only the coarse
//! property that survives a loaded CI runner: the miss is not an order of
//! magnitude cheaper than the hit.

use super::{attempt_login, setup_app};
use actix_web::http::StatusCode;
use r_data_core_persistence::{AdminUserRepository, AdminUserRepositoryTrait};
use r_data_core_test_support::{clear_test_db, create_test_admin_user};
use serial_test::serial;
use std::sync::Arc;
use std::time::{Duration, Instant};

const SAMPLES: u32 = 5;

async fn median_login_time<S>(app: &S, username: &str) -> Duration
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let mut samples = Vec::new();
    for _ in 0..SAMPLES {
        let started = Instant::now();
        let status = attempt_login(app, username, "definitely_the_wrong_password").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "both paths return 401");
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    samples[samples.len() / 2]
}

#[tokio::test]
#[serial]
async fn test_unknown_user_costs_comparable_time_to_a_known_one(
) -> r_data_core_core::error::Result<()> {
    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let known = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    // Measure the known user first; the account locks after the threshold but
    // every attempt still runs a full verification, which is what we time.
    let hit = median_login_time(&app, &known).await;
    let miss = median_login_time(&app, "user_that_does_not_exist").await;

    // Before the fix the miss was ~100x faster. A quarter of the hit time is a
    // wide margin that still fails loudly if the dummy verify is removed.
    assert!(
        miss * 4 >= hit,
        "unknown-user path is far cheaper than a real one \
         (miss {miss:?} vs hit {hit:?}) — the timing oracle is back"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

/// Both paths must also be indistinguishable by status code and body.
#[tokio::test]
#[serial]
async fn test_unknown_user_and_wrong_password_are_indistinguishable(
) -> r_data_core_core::error::Result<()> {
    use actix_web::test;

    let (app, pool) = setup_app().await?;

    let user_uuid = create_test_admin_user(&pool).await?;
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let known = repo.find_by_uuid(&user_uuid).await?.unwrap().username;

    let mut bodies = Vec::new();
    for username in [known.as_str(), "user_that_does_not_exist"] {
        let req = test::TestRequest::post()
            .uri("/admin/api/v1/auth/login")
            .set_json(
                serde_json::json!({ "username": username, "password": "wrong_but_long_enough" }),
            )
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let mut body: serde_json::Value = test::read_body_json(resp).await;
        // Request id and timestamp differ by design.
        if let Some(meta) = body.get_mut("meta") {
            *meta = serde_json::Value::Null;
        }
        bodies.push(body);
    }

    assert_eq!(
        bodies[0], bodies[1],
        "response bodies must not reveal whether the account exists"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
