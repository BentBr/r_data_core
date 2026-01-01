#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use httpmock::MockServer;
use r_data_core_core::config::LicenseConfig;
use r_data_core_persistence::StatisticsRepository;
use r_data_core_services::StatisticsService;
use r_data_core_test_support::setup_test_db;
use std::sync::Arc;

#[tokio::test]
#[serial_test::serial]
async fn test_statistics_collection() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool;

    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/submit");
        then.status(200)
            .json_body(serde_json::json!({ "success": true }));
    });

    let config = LicenseConfig {
        license_key: None,
        private_key: None,
        verification_url: format!("http://{}/verify", mock_server.address()),
        statistics_url: format!("http://{}/submit", mock_server.address()),
    };

    let repository = Arc::new(StatisticsRepository::new(pool));
    let service = StatisticsService::new(config, repository);

    // Collect and send statistics (silent failure)
    service
        .collect_and_send(
            "http://localhost:8080",
            &["http://localhost:3000".to_string()],
        )
        .await;

    // Verify the mock was called
    mock.assert();
}

#[tokio::test]
#[serial_test::serial]
async fn test_statistics_collection_silent_failure() {
    let test_db = setup_test_db().await;
    let pool = test_db.pool;

    // Mock server that returns error
    let mock_server = MockServer::start();
    let _mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/submit");
        then.status(500).body("Internal Server Error");
    });

    let config = LicenseConfig {
        license_key: None,
        private_key: None,
        verification_url: format!("http://{}/verify", mock_server.address()),
        statistics_url: format!("http://{}/submit", mock_server.address()),
    };

    let repository = Arc::new(StatisticsRepository::new(pool));
    let service = StatisticsService::new(config, repository);

    // Should not panic on error (silent failure)
    service
        .collect_and_send(
            "http://localhost:8080",
            &["http://localhost:3000".to_string()],
        )
        .await;

    // Test passes if no panic occurs
}
