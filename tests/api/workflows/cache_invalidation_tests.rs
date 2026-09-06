#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! A cached workflow must not outlive an edit.
//!
//! The workflow config carries the public endpoint's pre-shared key and its
//! rate limit, so a stale entry either throttles a workflow the admin just
//! opened up, or keeps honouring a key they just rotated away.

use std::sync::Arc;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_services::{WorkflowRepositoryAdapter, WorkflowService};
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;

use super::common::create_provider_workflow;

/// The smallest config `WorkflowService::update` will accept — it validates the
/// DSL, unlike the repository-level create helper.
fn minimal_valid_config() -> serde_json::Value {
    serde_json::json!({
        "steps": [{
            "from": {
                "type": "format",
                "source": {
                    "source_type": "uri",
                    "config": { "uri": "http://example.com/data.csv" },
                    "auth": { "type": "none" }
                },
                "format": { "format_type": "csv", "options": { "has_header": true, "delimiter": "," } },
                "mapping": { "price": "price" }
            },
            "transform": { "type": "none" },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": { "format_type": "json", "options": {} },
                "mapping": { "price": "entity.total" }
            }
        }]
    })
}

/// A service with caching switched on — the whole point of these tests.
fn cached_service(pool: &sqlx::PgPool) -> WorkflowService {
    let cache = Arc::new(CacheManager::new(CacheConfig {
        entity_definition_ttl: 0,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 1000,
    }));

    let adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(pool.clone()));
    WorkflowService::new(Arc::new(adapter)).with_cache(cache)
}

#[tokio::test]
#[serial]
async fn test_a_cached_workflow_is_served_from_cache() -> anyhow::Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let creator = create_test_admin_user(&pool.pool).await?;
    let uuid =
        create_provider_workflow(&pool.pool, creator, serde_json::json!({ "steps": [] })).await?;

    let service = cached_service(&pool.pool);

    let first = service.get(uuid).await?.expect("workflow exists");
    let second = service
        .get(uuid)
        .await?
        .expect("still there on the second read");

    assert_eq!(first.uuid, second.uuid);
    assert_eq!(first.name, second.name);

    clear_test_db(&pool).await?;
    Ok(())
}

/// The regression that matters: an update must not leave the old config behind.
#[tokio::test]
#[serial]
async fn test_updating_a_workflow_invalidates_the_cache() -> anyhow::Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let creator = create_test_admin_user(&pool.pool).await?;
    let uuid = create_provider_workflow(&pool.pool, creator, minimal_valid_config()).await?;

    let service = cached_service(&pool.pool);

    // Populate the cache.
    let before = service.get(uuid).await?.expect("workflow exists");

    // Edit it through the same service, which must drop the entry.
    let update = r_data_core_workflow::data::requests::UpdateWorkflowRequest {
        name: format!("{}-renamed", before.name),
        description: before.description.clone(),
        kind: before.kind.to_string(),
        enabled: before.enabled,
        schedule_cron: before.schedule_cron.clone(),
        config: minimal_valid_config(),
        versioning_disabled: before.versioning_disabled,
    };
    service.update(uuid, &update, creator).await?;

    let after = service.get(uuid).await?.expect("workflow still exists");
    assert_eq!(
        after.name, update.name,
        "a cached copy outlived the update — the edit is invisible to readers"
    );

    clear_test_db(&pool).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_deleting_a_workflow_invalidates_the_cache() -> anyhow::Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let creator = create_test_admin_user(&pool.pool).await?;
    let uuid =
        create_provider_workflow(&pool.pool, creator, serde_json::json!({ "steps": [] })).await?;

    let service = cached_service(&pool.pool);
    service.get(uuid).await?.expect("workflow exists");

    service.delete(uuid, creator).await?;

    assert!(
        service.get(uuid).await?.is_none(),
        "a deleted workflow must not keep answering from cache"
    );

    clear_test_db(&pool).await?;
    Ok(())
}
