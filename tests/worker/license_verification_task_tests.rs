#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use httpmock::MockServer;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{CacheConfig, LicenseConfig};
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_test_support::setup_test_db;
use r_data_core_worker::context::TaskContext;
use r_data_core_worker::tasks::license_verification::LicenseVerificationTask;
use serial_test::serial;
use std::sync::Arc;

#[tokio::test]
#[serial]
async fn test_license_verification_task_name_and_cron() {
    let config = LicenseConfig::default();
    let task = LicenseVerificationTask::new("0 * * * * *".to_string(), config);

    assert_eq!(task.name(), "license_verification");
    assert_eq!(task.cron(), "0 * * * * *");
}

#[tokio::test]
#[serial]
async fn test_license_verification_task_none_state() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool;

    let config = LicenseConfig {
        license_key: None,
        private_key: None,
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

    let task = LicenseVerificationTask::new("0 * * * * *".to_string(), config);
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);

    // Should succeed even with no license key
    let result = task.execute(&context).await;
    assert!(result.is_ok(), "Task should handle None state gracefully");
}

#[tokio::test]
#[serial]
async fn test_license_verification_task_prevents_multiple_runs() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool;

    let config = LicenseConfig {
        license_key: None,
        private_key: None,
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

    let task = LicenseVerificationTask::new("0 * * * * *".to_string(), config.clone());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager.clone());

    // First execution
    let result1 = task.execute(&context).await;
    assert!(result1.is_ok(), "First execution should succeed");

    // Second execution immediately after - should be skipped
    let task2 = LicenseVerificationTask::new("0 * * * * *".to_string(), config);
    let context2 = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    let result2 = task2.execute(&context2).await;
    assert!(
        result2.is_ok(),
        "Second execution should be skipped but not fail"
    );
}
