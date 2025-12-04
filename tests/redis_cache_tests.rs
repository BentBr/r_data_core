#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_core::cache::backend::CacheBackend;
use r_data_core_core::cache::redis::RedisCache;
use serde_json::json;
use uuid::Uuid;

async fn get_test_cache() -> Option<RedisCache> {
    let url = std::env::var("REDIS_URL").ok()?;
    RedisCache::new(&url, 3600).await.ok()
}

#[tokio::test]
async fn test_redis_cache_connection_if_available() {
    let Some(cache) = get_test_cache().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    // Test that we can perform operations (connection works)
    let test_key = format!("test:connection:{}", Uuid::now_v7());
    let test_data = json!({
        "id": Uuid::now_v7(),
        "name": "test",
        "value": 42
    });

    // Set and get should work
    cache
        .set(&test_key, &test_data, Some(60))
        .await
        .expect("set should succeed");

    let retrieved: Option<serde_json::Value> =
        cache.get(&test_key).await.expect("get should succeed");

    assert_eq!(retrieved, Some(test_data));

    // Cleanup
    cache
        .delete(&test_key)
        .await
        .expect("delete should succeed");
}

#[tokio::test]
async fn test_redis_cache_ping_connection_if_available() {
    let Some(cache) = get_test_cache().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    // The cache should have been initialized with a successful PING
    // This test verifies that the connection test in RedisCache::new works
    // with the updated redis 0.32 API
    let test_key = format!("test:ping:{}", Uuid::now_v7());
    let test_data = json!({
        "id": Uuid::now_v7(),
        "name": "ping_test",
        "value": 100
    });

    // If we can set/get, the connection is working
    cache
        .set(&test_key, &test_data, Some(10))
        .await
        .expect("set should succeed after successful connection");

    let retrieved: Option<serde_json::Value> =
        cache.get(&test_key).await.expect("get should succeed");
    assert_eq!(retrieved, Some(test_data));

    // Cleanup
    cache
        .delete(&test_key)
        .await
        .expect("delete should succeed");
}

#[tokio::test]
async fn test_redis_cache_query_async_api_if_available() {
    let Some(cache) = get_test_cache().await else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    // Test that the updated query_async API works correctly
    // This verifies the migration from query_async::<T, C> to query_async with type annotation
    let test_key = format!("test:query_api:{}", Uuid::now_v7());
    let test_data = json!({
        "id": Uuid::now_v7(),
        "name": "query_test",
        "value": 200
    });

    // These operations use the updated redis 0.32 API internally
    cache
        .set(&test_key, &test_data, Some(30))
        .await
        .expect("set should work with updated API");

    let retrieved: Option<serde_json::Value> = cache.get(&test_key).await.expect("get should work");
    assert_eq!(retrieved, Some(test_data.clone()));

    // Test delete_by_prefix which uses SCAN and DEL commands
    let prefix = format!("test:query_api:{}", Uuid::now_v7());
    let test_data2 = test_data.clone();
    cache
        .set(&format!("{prefix}:1"), &test_data2, Some(30))
        .await
        .expect("set should work");
    let test_data3 = test_data.clone();
    cache
        .set(&format!("{prefix}:2"), &test_data3, Some(30))
        .await
        .expect("set should work");

    let deleted = cache
        .delete_by_prefix(&prefix)
        .await
        .expect("delete_by_prefix should work with updated API");
    assert_eq!(deleted, 2);

    // Cleanup
    cache
        .delete(&test_key)
        .await
        .expect("delete should succeed");
}
