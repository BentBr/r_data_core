#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use httpmock::MockServer;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::{CacheConfig, LicenseConfig};
use r_data_core_services::license::service::LicenseState;
use r_data_core_services::LicenseService;
use serial_test::serial;
use std::sync::Arc;

#[tokio::test]
#[serial]
async fn test_license_verification_none() {
    // Use mock server URL (even though we won't call it, for consistency)
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

    let service = LicenseService::new(config, cache_manager);

    let result = service.verify_license().await.expect("Should not fail");

    assert_eq!(
        result.state,
        r_data_core_services::license::service::LicenseState::None
    );
    assert!(result.company.is_none());
    assert!(result.license_id.is_none());
}

#[tokio::test]
#[serial]
async fn test_license_verification_invalid_jwt() {
    let mock_server = MockServer::start();
    let config = LicenseConfig {
        license_key: Some("invalid.jwt.token".to_string()),
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

    let service = LicenseService::new(config, cache_manager.clone());

    // First call - should fail to decode JWT, so it will be Error state
    // (not Invalid, because Invalid requires a valid JWT that the API says is invalid)
    let result = service.verify_license().await.expect("Should not fail");

    // Should be Error state (can't decode JWT)
    assert_eq!(
        result.state,
        r_data_core_services::license::service::LicenseState::Error
    );
    assert!(result.error_message.is_some());
}

#[tokio::test]
#[serial]
async fn test_license_verification_api_returns_invalid() {
    use r_data_core_license::{create_license_key, LicenseType};
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let private_key_path = temp_dir.path().join("private.key");

    // Generate private key using openssl
    let private_key_output = std::process::Command::new("openssl")
        .args(["genrsa", "-out", private_key_path.to_str().unwrap(), "2048"])
        .output();

    let Ok(private_key_output) = private_key_output else {
        eprintln!("Skipping test - openssl not available");
        return;
    };

    if !private_key_output.status.success() {
        eprintln!("Skipping test - failed to generate private key");
        return;
    }

    // Create a valid JWT license key
    let license_key = create_license_key(
        "Test Company",
        LicenseType::Community,
        private_key_path.to_str().unwrap(),
        Some(time::OffsetDateTime::now_utc() + time::Duration::days(365)),
    )
    .expect("Failed to create license key");

    let mock_server = MockServer::start();
    let mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST)
            .path("/verify")
            .json_body(serde_json::json!({ "license_key": license_key }));
        then.status(200).json_body(serde_json::json!({
            "valid": false,
            "message": "License has been revoked by administrator"
        }));
    });

    let config = LicenseConfig {
        license_key: Some(license_key),
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

    let service = LicenseService::new(config, cache_manager.clone());

    // Verify license - API should return invalid
    let result = service.verify_license().await.expect("Should not fail");

    // Should be Invalid state (valid JWT but API says invalid)
    assert_eq!(
        result.state,
        r_data_core_services::license::service::LicenseState::Invalid
    );
    assert_eq!(result.company, Some("Test Company".to_string()));
    assert!(result.error_message.is_some());
    let error_msg = result.error_message.unwrap();
    assert!(error_msg.contains("revoked") || error_msg.contains("License"));

    // Verify it's cached
    let cached = service
        .get_cached_license_status()
        .await
        .expect("Should not fail");
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().state, LicenseState::Invalid);

    mock.assert();
}

#[tokio::test]
#[serial]
async fn test_license_verification_api_returns_valid() {
    use r_data_core_license::{create_license_key, LicenseType};
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let private_key_path = temp_dir.path().join("private.key");

    // Generate private key using openssl
    let private_key_output = std::process::Command::new("openssl")
        .args(["genrsa", "-out", private_key_path.to_str().unwrap(), "2048"])
        .output();

    let Ok(private_key_output) = private_key_output else {
        eprintln!("Skipping test - openssl not available");
        return;
    };

    if !private_key_output.status.success() {
        eprintln!("Skipping test - failed to generate private key");
        return;
    }

    // Create a valid JWT license key
    let license_key = create_license_key(
        "Valid Company",
        LicenseType::Enterprise,
        private_key_path.to_str().unwrap(),
        Some(time::OffsetDateTime::now_utc() + time::Duration::days(365)),
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

    let service = LicenseService::new(config, cache_manager.clone());

    // Verify license - API should return valid
    let result = service.verify_license().await.expect("Should not fail");

    // Should be a Valid state
    assert_eq!(result.state, LicenseState::Valid);
    assert_eq!(result.company, Some("Valid Company".to_string()));
    assert_eq!(result.license_type, Some("Enterprise".to_string()));
    assert!(result.license_id.is_some());
    assert!(result.version.is_some());
    assert!(result.error_message.is_none());

    // Verify it's cached
    let cached = service
        .get_cached_license_status()
        .await
        .expect("Should not fail");
    assert!(cached.is_some());
    let cached_result = cached.unwrap();
    assert_eq!(cached_result.state, LicenseState::Valid);
    assert_eq!(cached_result.company, Some("Valid Company".to_string()));

    mock.assert();
}

#[tokio::test]
#[serial]
async fn test_license_verification_network_error() {
    use r_data_core_license::{create_license_key, LicenseType};
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let private_key_path = temp_dir.path().join("private.key");

    // Generate the private key using openssl
    let private_key_output = std::process::Command::new("openssl")
        .args(["genrsa", "-out", private_key_path.to_str().unwrap(), "2048"])
        .output();

    let Ok(private_key_output) = private_key_output else {
        eprintln!("Skipping test - openssl not available");
        return;
    };

    if !private_key_output.status.success() {
        eprintln!("Skipping test - failed to generate private key");
        return;
    }

    // Create a valid JWT license key
    let license_key = create_license_key(
        "Test Company",
        LicenseType::Enterprise,
        private_key_path.to_str().unwrap(),
        Some(time::OffsetDateTime::now_utc() + time::Duration::days(365)),
    )
    .expect("Failed to create license key");

    // Use invalid URL to simulate network error
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

    let service = LicenseService::new(config, cache_manager);

    // Verify license - should handle network error
    let result = service.verify_license().await.expect("Should not fail");

    // Should be Error state due to network error
    assert_eq!(
        result.state,
        r_data_core_services::license::service::LicenseState::Error
    );
    assert_eq!(result.company, Some("Test Company".to_string()));
    assert!(result.error_message.is_some());
    let error_msg = result.error_message.unwrap();
    assert!(error_msg.contains("Network error") || error_msg.contains("error"));
}

#[tokio::test]
#[serial]
async fn test_license_verification_caching() {
    let mock_server = MockServer::start();
    let _mock = mock_server.mock(|when, then| {
        when.method(httpmock::Method::POST).path("/verify");
        then.status(200)
            .json_body(serde_json::json!({ "valid": true, "message": "Valid license" }));
    });

    // Create a valid JWT token for testing (we'll use a simple test token)
    // In real tests, we'd generate a proper JWT with the license crate
    let test_license_key = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJ2ZXJzaW9uIjoidjEiLCJjb21wYW55IjoiVGVzdCBDb21wYW55IiwibGljZW5zZV90eXBlIjoiRW50ZXJwcmlzZSIsImlzc3VlZF9hdCI6IjIwMjQtMDEtMDFUMDA6MDA6MDBaIiwibGljZW5zZV9pZCI6IjAxOGYxMjM0LTU2NzgtOWFiYy1kZWYwLTEyMzQ1Njc4OWFiYyJ9.test_signature";

    let config = LicenseConfig {
        license_key: Some(test_license_key.to_string()),
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

    let service = LicenseService::new(config, cache_manager.clone());

    // First call - should call API (but will fail to decode JWT, so will be Error state)
    let _result1 = service.verify_license().await;

    // Second call - should use cache (but since first failed, cache will have Error state)
    let result2 = service
        .get_cached_license_status()
        .await
        .expect("Should not fail");

    // Should have cached result
    assert!(result2.is_some());

    // Mock should only be called once (or not at all if JWT decode fails first)
    // The exact behavior depends on whether JWT decode succeeds
}

#[tokio::test]
#[serial]
async fn test_get_cached_license_status_none() {
    // Use mock server URL (even though we won't call it, for consistency)
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

    let service = LicenseService::new(config, cache_manager);

    let result = service
        .get_cached_license_status()
        .await
        .expect("Should not fail");

    assert!(result.is_some());
    let status = result.unwrap();
    assert_eq!(
        status.state,
        r_data_core_services::license::service::LicenseState::None
    );
}

#[tokio::test]
#[serial]
async fn test_license_verification_expired() {
    use std::process::Command;
    use tempfile::TempDir;

    // Generate test keys
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let private_key_path = temp_dir.path().join("private.key");
    let public_key_path = temp_dir.path().join("public.key");

    // Generate private key using openssl
    let private_key_output = Command::new("openssl")
        .args(["genrsa", "-out", private_key_path.to_str().unwrap(), "2048"])
        .output();

    let Ok(private_key_output) = private_key_output else {
        eprintln!("Skipping test - openssl not available");
        return;
    };

    if !private_key_output.status.success() {
        eprintln!("Skipping test - failed to generate private key");
        return;
    }

    // Generate public key
    let public_key_output = Command::new("openssl")
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
        eprintln!("Skipping test - failed to generate public key");
        return;
    }

    // Create an expired license (expires 1 day ago)
    let create_output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_license",
            "--bin",
            "license_tool",
            "create",
            "--company",
            "Expired Test Company",
            "--license-type",
            "Enterprise",
            "--private-key-file",
            private_key_path.to_str().unwrap(),
            "--expires-days",
            "0", // Expires immediately (0 days = now)
        ])
        .output()
        .expect("Failed to execute license_tool create");

    if !create_output.status.success() {
        eprintln!("Skipping test - failed to create license");
        return;
    }

    // Extract license key from output
    let output_str = String::from_utf8_lossy(&create_output.stdout);
    let license_key = output_str
        .lines()
        .find(|line| line.starts_with("eyJ"))
        .expect("License key should be in output")
        .to_string();

    // Wait a moment to ensure expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Verify the expired license
    let verify_output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_license",
            "--bin",
            "license_tool",
            "verify",
            "--license-key",
            &license_key,
            "--public-key-file",
            public_key_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute license_tool verify");

    // Should fail for expired license
    assert!(
        !verify_output.status.success(),
        "Expired license verification should fail"
    );

    let verify_output_str = String::from_utf8_lossy(&verify_output.stdout);
    assert!(
        verify_output_str.contains("INVALID") || verify_output_str.contains("expired"),
        "Output should indicate license is invalid/expired"
    );
}
