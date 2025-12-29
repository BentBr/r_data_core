#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Integration tests for `CacheService`.

use std::sync::Arc;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_services::CacheService;
use uuid::Uuid;

const fn create_test_config() -> CacheConfig {
    CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 1000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    }
}

async fn get_cache_service_with_redis() -> Option<(CacheService, Arc<CacheManager>)> {
    let url = std::env::var("REDIS_URL").ok()?;
    let config = create_test_config();
    let manager = CacheManager::new(config).with_redis(&url).await.ok()?;
    let manager = Arc::new(manager);
    let service = CacheService::new(manager.clone());
    Some((service, manager))
}

#[tokio::test]
async fn test_cache_service_clear_all_with_redis() {
    let Some((service, manager)) = get_cache_service_with_redis().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    // Set some test values with unique prefixes
    let prefix = format!("test:clear_all:{}", Uuid::now_v7().simple());
    let key1 = format!("{prefix}:1");
    let key2 = format!("{prefix}:2");

    manager.set(&key1, &"value1", None).await.unwrap();
    manager.set(&key2, &"value2", None).await.unwrap();

    // Verify values exist
    let result: Option<String> = manager.get(&key1).await.unwrap();
    assert!(result.is_some());

    // Clear all cache
    service.clear_all().await.unwrap();

    // Values should be gone
    let result: Option<String> = manager.get(&key1).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cache_service_clear_by_prefix_with_redis() {
    let Some((service, manager)) = get_cache_service_with_redis().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    // Set some test values with different prefixes
    let test_id = Uuid::now_v7().simple().to_string();
    let entity_prefix = format!("test:entities:{test_id}:");
    let api_key_prefix = format!("test:api_keys:{test_id}:");

    manager
        .set(&format!("{entity_prefix}1"), &"entity1", None)
        .await
        .unwrap();
    manager
        .set(&format!("{entity_prefix}2"), &"entity2", None)
        .await
        .unwrap();
    manager
        .set(&format!("{api_key_prefix}1"), &"apikey1", None)
        .await
        .unwrap();

    // Clear only entity prefix
    let deleted = service.clear_by_prefix(&entity_prefix).await.unwrap();
    assert_eq!(deleted, 2);

    // Entity keys should be gone
    let result: Option<String> = manager.get(&format!("{entity_prefix}1")).await.unwrap();
    assert!(result.is_none());

    // API key should still exist
    let result: Option<String> = manager.get(&format!("{api_key_prefix}1")).await.unwrap();
    assert!(result.is_some());

    // Cleanup
    manager.delete(&format!("{api_key_prefix}1")).await.unwrap();
}

#[tokio::test]
async fn test_cache_service_with_in_memory_only() {
    let config = create_test_config();
    let manager = Arc::new(CacheManager::new(config));
    let service = CacheService::new(manager.clone());

    // Set some values
    let key1 = format!("test:inmem:{}:1", Uuid::now_v7().simple());
    let key2 = format!("test:inmem:{}:2", Uuid::now_v7().simple());

    manager.set(&key1, &"value1", None).await.unwrap();
    manager.set(&key2, &"value2", None).await.unwrap();

    // Clear all
    service.clear_all().await.unwrap();

    // Values should be gone
    let result: Option<String> = manager.get(&key1).await.unwrap();
    assert!(result.is_none());
}
