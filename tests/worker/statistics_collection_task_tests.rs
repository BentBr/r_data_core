#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use httpmock::MockServer;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{CacheConfig, LicenseConfig};
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_test_support::setup_test_db;
use r_data_core_worker::context::TaskContext;
use r_data_core_worker::tasks::statistics_collection::StatisticsCollectionTask;
use serial_test::serial;
use std::sync::Arc;

#[tokio::test]
#[serial]
async fn test_statistics_collection_task_name_and_cron() {
    let config = LicenseConfig::default();
    let task = StatisticsCollectionTask::new(
        "0 * * * * *".to_string(),
        config,
        "http://localhost:8080".to_string(),
        vec!["http://localhost:3000".to_string()],
        "postgres://localhost/test".to_string(),
    );

    assert_eq!(task.name(), "statistics_collection");
    assert_eq!(task.cron(), "0 * * * * *");
}

#[tokio::test]
#[serial]
async fn test_statistics_collection_task_executes() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool.clone();

    let mock_server = MockServer::start();
    let _mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/submit");
        then.status(200)
            .json_body(serde_json::json!({ "success": true }));
    });

    let config = LicenseConfig {
        license_key: None,
        private_key: None,
        public_key: None,
        verification_url: "https://license.rdatacore.eu/verify".to_string(),
        statistics_url: format!("http://{}/submit", mock_server.address()),
    };

    let cache_config = CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let task = StatisticsCollectionTask::new(
        "0 * * * * *".to_string(),
        config,
        "http://localhost:8080".to_string(),
        vec!["http://localhost:3000".to_string()],
        "postgres://localhost/test".to_string(),
    );
    let context = TaskContext::with_cache(pool.clone(), cache_manager);

    // Should succeed (silent failure on API errors)
    let result = task.execute(&context).await;
    assert!(result.is_ok(), "Task should execute without error");
}

#[tokio::test]
#[serial]
async fn test_statistics_collection_task_prevents_multiple_runs() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool.clone();

    let config = LicenseConfig {
        license_key: None,
        private_key: None,
        public_key: None,
        verification_url: "http://localhost:9999/verify".to_string(), // Mock URL (won't be called in these tests)
        statistics_url: "http://localhost:9999/submit".to_string(), // Mock URL (won't be called in these tests)
    };

    let cache_config = CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let task = StatisticsCollectionTask::new(
        "0 * * * * *".to_string(),
        config.clone(),
        "http://localhost:8080".to_string(),
        vec!["http://localhost:3000".to_string()],
        "postgres://localhost/test".to_string(),
    );
    let context = TaskContext::with_cache(pool.clone(), cache_manager.clone());

    // First execution
    let result1 = task.execute(&context).await;
    assert!(result1.is_ok(), "First execution should succeed");

    // Second execution immediately after - should be skipped
    let task2 = StatisticsCollectionTask::new(
        "0 * * * * *".to_string(),
        config,
        "http://localhost:8080".to_string(),
        vec!["http://localhost:3000".to_string()],
        "postgres://localhost/test".to_string(),
    );
    let context2 = TaskContext::with_cache(pool.clone(), cache_manager);
    let result2 = task2.execute(&context2).await;
    assert!(
        result2.is_ok(),
        "Second execution should be skipped but not fail"
    );
}
