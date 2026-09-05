#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! Every repository entry point against an unreachable database.
//!
//! The persistence crate denies `unwrap`/`expect`/`panic`, so losing the
//! database must surface as an `Err` from each method rather than taking the
//! process down — a worker that panics on a dropped connection stops draining
//! its queue entirely. These are the `map_err` arms the happy-path tests never
//! reach.

use r_data_core_persistence::{
    ApiKeyRepository, ApiKeyRepositoryTrait, ComponentVersionRepository, DashboardStatsRepository,
    DashboardStatsRepositoryTrait, EmailTemplateRepository, EmailTemplateRepositoryTrait,
    PasswordResetRepository, PasswordResetRepositoryTrait,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// A pool pointed at a port nothing listens on. `connect_lazy` defers the
/// failure to first use, so construction succeeds and every query fails.
fn dead_pool() -> Pool<Postgres> {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(300))
        .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/nonexistent")
        .expect("a lazy pool is built without contacting the server")
}

#[tokio::test]
async fn test_api_key_repository_errors_rather_than_panics() {
    let repo = ApiKeyRepository::new(Arc::new(dead_pool()));
    let uuid = Uuid::now_v7();

    assert!(repo.get_api_key_roles(uuid).await.is_err());
    assert!(repo.get_api_keys_by_role(uuid).await.is_err());
    assert!(repo.assign_role(uuid, uuid).await.is_err());
    assert!(repo.unassign_role(uuid, uuid).await.is_err());
    assert!(repo.update_api_key_roles(uuid, &[uuid]).await.is_err());
    assert!(repo.get_by_uuid(uuid).await.is_err());
    assert!(repo.list_by_user(uuid, 10, 0, None, None).await.is_err());
    assert!(repo.count_by_user(uuid).await.is_err());
    assert!(repo.revoke(uuid).await.is_err());
    assert!(repo.get_by_name(uuid, "name").await.is_err());
}

#[tokio::test]
async fn test_email_template_repository_errors_rather_than_panics() {
    let repo = EmailTemplateRepository::new(dead_pool());
    let uuid = Uuid::now_v7();

    assert!(repo.list_all().await.is_err());
    assert!(repo.get_by_uuid(uuid).await.is_err());
    assert!(repo.get_by_slug("slug").await.is_err());
    assert!(repo.delete(uuid).await.is_err());
}

#[tokio::test]
async fn test_component_version_repository_errors_rather_than_panics() {
    let repo = ComponentVersionRepository::new(dead_pool());

    assert!(repo.get_all().await.is_err());
    assert!(repo.get("backend").await.is_err());
    assert!(repo.upsert("backend", "0.0.0").await.is_err());
}

#[tokio::test]
async fn test_dashboard_stats_repository_errors_rather_than_panics() {
    let repo = DashboardStatsRepository::new(dead_pool());

    assert!(repo.get_dashboard_stats().await.is_err());
}

#[tokio::test]
async fn test_password_reset_repository_errors_rather_than_panics() {
    let repo = PasswordResetRepository::new(dead_pool());
    let uuid = Uuid::now_v7();

    assert!(repo.find_by_token_hash("hash").await.is_err());
    assert!(repo.find_latest_for_user(uuid).await.is_err());
    assert!(repo.mark_used(uuid).await.is_err());
    assert!(repo.delete_expired().await.is_err());
    assert!(repo.delete_for_user(uuid).await.is_err());
    assert!(repo
        .insert_token(uuid, "hash", OffsetDateTime::now_utc())
        .await
        .is_err());
}
