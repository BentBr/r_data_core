#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use httpmock::MockServer;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{CacheConfig, LicenseConfig};
use r_data_core_license::LicenseToolService;
use r_data_core_license::{create_license_key, LicenseType};
use r_data_core_services::license::service::{LicenseService, LicenseState};
use serial_test::serial;
use std::sync::Arc;
use tempfile::TempDir;

/// Generate RSA key pair for testing
fn generate_test_keys(temp_dir: &TempDir) -> (String, String) {
    let private_key_path = temp_dir.path().join("private.key");
    let public_key_path = temp_dir.path().join("public.key");

    // Generate private key using openssl
    let private_key_output = std::process::Command::new("openssl")
        .args(["genrsa", "-out", private_key_path.to_str().unwrap(), "2048"])
        .output();

    let Ok(private_key_output) = private_key_output else {
        return (String::new(), String::new());
    };

    if !private_key_output.status.success() {
        return (String::new(), String::new());
    }

    // Generate public key from private key
    let public_key_output = std::process::Command::new("openssl")
        .args([
            "rsa",
            "-in",
            private_key_path.to_str().unwrap(),
            "-pubout",
            "-out",
            public_key_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to generate public key");

    if !public_key_output.status.success() {
        return (String::new(), String::new());
    }

    let private_key =
        std::fs::read_to_string(&private_key_path).expect("Failed to read private key");
    let public_key = std::fs::read_to_string(&public_key_path).expect("Failed to read public key");

    (private_key, public_key)
}

#[tokio::test]
#[serial]
async fn test_license_check_valid() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let (private_key, _) = generate_test_keys(&temp_dir);

    if private_key.is_empty() {
        eprintln!("Skipping test - openssl not available");
        return;
    }

    let private_key_path = temp_dir.path().join("private.key");
    std::fs::write(&private_key_path, private_key).expect("Failed to write private key");

    // Create a valid license key
    let license_key = create_license_key(
        "Test Company",
        LicenseType::Enterprise,
        private_key_path.to_str().unwrap(),
        Some(365),
    )
    .expect("Failed to create license key");

    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST)
            .path("/verify")
            .json_body(serde_json::json!({ "license_key": license_key }));
        then.status(200)
            .json_body(serde_json::json!({ "valid": true }));
    });

    let config = LicenseConfig {
        license_key: Some(license_key.clone()),
        private_key: None,
        public_key: None,
        verification_url: format!("http://{}/verify", mock_server.address()),
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

    // Test check_license function
    let result = LicenseToolService::check_license(config, cache_manager.clone())
        .await
        .expect("Should not fail");

    assert_eq!(result.state, r_data_core_license::LicenseCheckState::Valid);
    assert_eq!(result.company, Some("Test Company".to_string()));
    assert_eq!(result.license_type, Some("Enterprise".to_string()));
    assert!(result.license_id.is_some());
    assert!(result.version.is_some());

    // Verify cache was updated
    let service = LicenseService::new(
        LicenseConfig {
            license_key: Some(license_key),
            private_key: None,
            public_key: None,
            verification_url: format!("http://{}/verify", mock_server.address()),
            statistics_url: format!("http://{}/submit", mock_server.address()),
        },
        cache_manager,
    );

    let cached_result = service
        .get_cached_license_status()
        .await
        .expect("Should not fail");

    assert!(cached_result.is_some());
    let cached = cached_result.unwrap();
    assert_eq!(cached.state, LicenseState::Valid);
    assert_eq!(cached.company, Some("Test Company".to_string()));

    mock.assert();
}

#[tokio::test]
#[serial]
async fn test_license_check_invalid_api_response() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let (private_key, _) = generate_test_keys(&temp_dir);

    if private_key.is_empty() {
        eprintln!("Skipping test - openssl not available");
        return;
    }

    let private_key_path = temp_dir.path().join("private.key");
    std::fs::write(&private_key_path, private_key).expect("Failed to write private key");

    // Create a valid license key
    let license_key = create_license_key(
        "Test Company",
        LicenseType::Community,
        private_key_path.to_str().unwrap(),
        Some(365),
    )
    .expect("Failed to create license key");

    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST)
            .path("/verify")
            .json_body(serde_json::json!({ "license_key": license_key }));
        then.status(200).json_body(serde_json::json!({
            "valid": false,
            "message": "License has been revoked"
        }));
    });

    let config = LicenseConfig {
        license_key: Some(license_key.clone()),
        private_key: None,
        public_key: None,
        verification_url: format!("http://{}/verify", mock_server.address()),
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

    // Test check_license function
    let result = LicenseToolService::check_license(config.clone(), cache_manager.clone())
        .await
        .expect("Should not fail");

    assert_eq!(
        result.state,
        r_data_core_license::LicenseCheckState::Invalid
    );
    assert_eq!(result.company, Some("Test Company".to_string()));
    assert!(result.error_message.is_some());
    let error_msg = result.error_message.unwrap();
    assert!(error_msg.contains("revoked") || error_msg.contains("License"));

    // Verify cache was updated with Invalid state
    let service = LicenseService::new(config, cache_manager);
    let cached_result = service
        .get_cached_license_status()
        .await
        .expect("Should not fail");

    assert!(cached_result.is_some());
    let cached = cached_result.unwrap();
    assert_eq!(cached.state, LicenseState::Invalid);

    mock.assert();
}

#[tokio::test]
#[serial]
async fn test_license_check_network_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let (private_key, _) = generate_test_keys(&temp_dir);

    if private_key.is_empty() {
        eprintln!("Skipping test - openssl not available");
        return;
    }

    let private_key_path = temp_dir.path().join("private.key");
    std::fs::write(&private_key_path, private_key).expect("Failed to write private key");

    // Create a valid license key
    let license_key = create_license_key(
        "Test Company",
        LicenseType::Enterprise,
        private_key_path.to_str().unwrap(),
        Some(365),
    )
    .expect("Failed to create license key");

    // Use a non-existent server URL to simulate network error
    let config = LicenseConfig {
        license_key: Some(license_key),
        private_key: None,
        public_key: None,
        verification_url: "http://127.0.0.1:99999/verify".to_string(), // Invalid port
        statistics_url: "http://127.0.0.1:99999/submit".to_string(),
    };

    let cache_config = CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    // Test check_license function - should handle network error gracefully
    // Note: The API call will fail, but we should still get a result
    let result = LicenseToolService::check_license(config, cache_manager)
        .await
        .expect("Should not fail");

    // Network errors result in Error state
    assert_eq!(result.state, r_data_core_license::LicenseCheckState::Error);
    assert!(result.error_message.is_some());
}

#[tokio::test]
#[serial]
async fn test_license_check_none_state() {
    let mock_server = MockServer::start();
    let config = LicenseConfig {
        license_key: None,
        private_key: None,
        public_key: None,
        verification_url: format!("http://{}/verify", mock_server.address()),
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

    let result = LicenseToolService::check_license(config, cache_manager)
        .await
        .expect("Should not fail");

    assert_eq!(result.state, r_data_core_license::LicenseCheckState::None);
    assert!(result.company.is_none());
    assert!(result.license_id.is_none());
}

#[tokio::test]
#[serial]
async fn test_license_check_cache_invalidation_with_new_timestamp() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let (private_key, _) = generate_test_keys(&temp_dir);

    if private_key.is_empty() {
        eprintln!("Skipping test - openssl not available");
        return;
    }

    let private_key_path = temp_dir.path().join("private.key");
    std::fs::write(&private_key_path, private_key).expect("Failed to write private key");

    // Create a valid license key
    let license_key = create_license_key(
        "Test Company",
        LicenseType::Enterprise,
        private_key_path.to_str().unwrap(),
        Some(365),
    )
    .expect("Failed to create license key");

    let mock_server = MockServer::start();

    // First mock: return valid
    let mock_valid = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST)
            .path("/verify")
            .json_body(serde_json::json!({ "license_key": license_key }));
        then.status(200)
            .json_body(serde_json::json!({ "valid": true }));
    });

    let config = LicenseConfig {
        license_key: Some(license_key.clone()),
        private_key: None,
        public_key: None,
        verification_url: format!("http://{}/verify", mock_server.address()),
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

    // First verification - should cache Valid state
    let service1 = LicenseService::new(config.clone(), cache_manager.clone());
    let result1 = service1.verify_license().await.expect("Should not fail");
    assert_eq!(result1.state, LicenseState::Valid);
    let first_verified_at = result1.verified_at;
    mock_valid.assert();

    // Verify cache has Valid state with first timestamp
    let cached1 = service1
        .get_cached_license_status()
        .await
        .expect("Should not fail");
    assert_eq!(cached1.as_ref().unwrap().state, LicenseState::Valid);
    assert_eq!(cached1.as_ref().unwrap().verified_at, first_verified_at);

    // Wait a bit to ensure different timestamp
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Now simulate network error (unreachable endpoint)
    let config_with_error = LicenseConfig {
        license_key: Some(license_key.clone()),
        private_key: None,
        public_key: None,
        verification_url: "http://127.0.0.1:99999/verify".to_string(), // Invalid port
        statistics_url: "http://127.0.0.1:99999/submit".to_string(),
    };

    // Use check_license which should clear cache and force fresh verification
    // Even though it fails, it should update cache with Error state and NEW timestamp
    let result2 =
        LicenseToolService::check_license(config_with_error.clone(), cache_manager.clone())
            .await
            .expect("Should not fail");

    assert_eq!(result2.state, r_data_core_license::LicenseCheckState::Error);
    let second_verified_at = result2.verified_at;

    // Verify timestamp is NEW (different from first)
    assert!(
        second_verified_at > first_verified_at,
        "New verification should have later timestamp"
    );

    // Verify cache was updated with Error state and NEW timestamp
    let service2 = LicenseService::new(config_with_error, cache_manager);
    let cached2 = service2
        .get_cached_license_status()
        .await
        .expect("Should not fail");
    assert!(cached2.is_some());
    let cached_result = cached2.unwrap();
    assert_eq!(cached_result.state, LicenseState::Error);
    assert_eq!(
        cached_result.verified_at, second_verified_at,
        "Cache should have the new timestamp from check_license"
    );
    assert!(
        cached_result.verified_at > first_verified_at,
        "Cached timestamp should be newer than the original"
    );
}
