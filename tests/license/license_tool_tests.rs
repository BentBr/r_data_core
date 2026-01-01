#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Generate RSA key pair for testing
/// Note: This requires openssl to be installed on the system
fn generate_test_keys(temp_dir: &TempDir) -> (String, String) {
    let private_key_path = temp_dir.path().join("private.key");
    let public_key_path = temp_dir.path().join("public.key");

    // Generate private key using openssl
    let private_key_output = Command::new("openssl")
        .args(["genrsa", "-out", private_key_path.to_str().unwrap(), "2048"])
        .output();

    let private_key_output = match private_key_output {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Warning: openssl not available, skipping test: {e}");
            return (String::new(), String::new());
        }
    };

    if !private_key_output.status.success() {
        eprintln!("Warning: Failed to generate private key, skipping test");
        return (String::new(), String::new());
    }

    // Generate public key from private key
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
        eprintln!("Warning: Failed to generate public key, skipping test");
        return (String::new(), String::new());
    }

    let private_key = fs::read_to_string(&private_key_path).expect("Failed to read private key");
    let public_key = fs::read_to_string(&public_key_path).expect("Failed to read public key");

    (private_key, public_key)
}

#[test]
fn test_license_tool_create() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let (private_key, _) = generate_test_keys(&temp_dir);

    // Skip test if openssl is not available
    if private_key.is_empty() {
        eprintln!("Skipping test - openssl not available");
        return;
    }

    let private_key_path = temp_dir.path().join("private.key");
    fs::write(&private_key_path, private_key).expect("Failed to write private key");

    let output = temp_dir.path().join("license.key");

    // Run the binary
    let result = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_license",
            "--bin",
            "license_tool",
            "create",
            "--company",
            "Test Company",
            "--license-type",
            "Enterprise",
            "--private-key-file",
            private_key_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute license_tool binary");

    // Check that the command succeeded
    assert!(result.status.success(), "Binary should exit successfully");

    // Get the output as string
    let output_str = String::from_utf8(result.stdout).expect("Failed to convert output to string");

    // Verify the output contains expected elements
    assert!(
        output_str.contains("License Key Created:"),
        "Output should contain license creation message"
    );
    assert!(
        output_str.contains("Company: Test Company"),
        "Output should contain company name"
    );
    assert!(
        output_str.contains("Type: Enterprise"),
        "Output should contain license type"
    );
    assert!(
        output_str.contains("Version: v1"),
        "Output should contain version"
    );
    assert!(
        output_str.contains("License Key:"),
        "Output should contain license key"
    );

    // Verify license key file was created
    assert!(output.exists(), "License key file should be created");
    let license_key = fs::read_to_string(&output).expect("Failed to read license key");
    assert!(!license_key.is_empty(), "License key should not be empty");
}

#[test]
fn test_license_tool_verify() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let (private_key, public_key) = generate_test_keys(&temp_dir);

    // Skip test if openssl is not available
    if private_key.is_empty() || public_key.is_empty() {
        eprintln!("Skipping test - openssl not available");
        return;
    }

    let private_key_path = temp_dir.path().join("private.key");
    let public_key_path = temp_dir.path().join("public.key");
    fs::write(&private_key_path, private_key).expect("Failed to write private key");
    fs::write(&public_key_path, public_key).expect("Failed to write public key");

    let license_key_path = temp_dir.path().join("license.key");

    // First, create a license key
    let create_result = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_license",
            "--bin",
            "license_tool",
            "create",
            "--company",
            "Test Company",
            "--license-type",
            "Enterprise",
            "--private-key-file",
            private_key_path.to_str().unwrap(),
            "--output",
            license_key_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute license_tool create");

    assert!(
        create_result.status.success(),
        "License creation should succeed"
    );

    // Now verify it
    let verify_result = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_license",
            "--bin",
            "license_tool",
            "verify",
            "--license-key-file",
            license_key_path.to_str().unwrap(),
            "--public-key-file",
            public_key_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute license_tool verify");

    // Check that the command succeeded
    if !verify_result.status.success() {
        let stderr = String::from_utf8_lossy(&verify_result.stderr);
        let stdout = String::from_utf8_lossy(&verify_result.stdout);
        panic!("Binary should exit successfully. Stderr: {stderr}, Stdout: {stdout}");
    }

    // Get the output as string
    let output_str =
        String::from_utf8(verify_result.stdout).expect("Failed to convert output to string");

    // Verify the output contains expected elements
    assert!(
        output_str.contains("License Verification:"),
        "Output should contain verification message"
    );
    assert!(
        output_str.contains("Status: VALID"),
        "Output should indicate valid status"
    );
    assert!(
        output_str.contains("Company: Test Company"),
        "Output should contain company name"
    );
    assert!(
        output_str.contains("Type: Enterprise"),
        "Output should contain license type"
    );
    assert!(
        output_str.contains("Version: v1"),
        "Output should contain version"
    );
}

#[test]
fn test_license_tool_verify_invalid() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let (_, public_key) = generate_test_keys(&temp_dir);

    // Skip test if openssl is not available
    if public_key.is_empty() {
        eprintln!("Skipping test - openssl not available");
        return;
    }

    let public_key_path = temp_dir.path().join("public.key");
    fs::write(&public_key_path, public_key).expect("Failed to write public key");

    // Try to verify an invalid license key
    let verify_result = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_license",
            "--bin",
            "license_tool",
            "verify",
            "--license-key",
            "invalid.jwt.token",
            "--public-key-file",
            public_key_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute license_tool verify");

    // Check that the command failed
    assert!(
        !verify_result.status.success(),
        "Binary should fail for invalid key"
    );

    // Get the output as string
    let output_str =
        String::from_utf8(verify_result.stdout).expect("Failed to convert output to string");

    // Verify the output contains error message
    assert!(
        output_str.contains("Status: INVALID"),
        "Output should indicate invalid status"
    );
}
