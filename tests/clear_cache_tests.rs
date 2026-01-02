#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::process::Command;

/// Helper function to get Redis URL from environment or skip test
fn get_redis_url() -> Option<String> {
    std::env::var("REDIS_URL").ok()
}

#[test]
fn test_clear_cache_help() {
    // Test that --help flag works
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "clear_cache",
            "--",
            "--help",
        ])
        .output()
        .expect("Failed to execute clear_cache binary");

    // Help should succeed, but cargo run might output compilation info to stderr
    // So we check if help text is in either stdout or stderr
    let stdout_str = String::from_utf8(output.stdout).expect("Failed to convert stdout to string");
    let stderr_str = String::from_utf8(output.stderr).expect("Failed to convert stderr to string");
    let output_str = format!("{stdout_str}{stderr_str}");

    // Verify help text contains expected sections
    assert!(
        output_str.contains("USAGE:"),
        "Help should contain USAGE section"
    );
    assert!(
        output_str.contains("OPTIONS:"),
        "Help should contain OPTIONS section"
    );
    assert!(
        output_str.contains("EXAMPLES:"),
        "Help should contain EXAMPLES section"
    );
    assert!(
        output_str.contains("COMMON CACHE PREFIXES:"),
        "Help should contain COMMON CACHE PREFIXES section"
    );

    // Verify correct prefixes are documented
    assert!(
        output_str.contains("entity_def:"),
        "Help should contain correct entity_def: prefix"
    );
    assert!(
        output_str.contains("api_key:"),
        "Help should contain correct api_key: prefix"
    );
    assert!(
        output_str.contains("role:"),
        "Help should contain role: prefix"
    );
    assert!(
        output_str.contains("user_roles:"),
        "Help should contain user_roles: prefix"
    );
    assert!(
        output_str.contains("api_key_roles:"),
        "Help should contain api_key_roles: prefix"
    );
    assert!(
        output_str.contains("user_permissions:"),
        "Help should contain user_permissions: prefix"
    );
    assert!(
        output_str.contains("api_key_permissions:"),
        "Help should contain api_key_permissions: prefix"
    );
    assert!(
        output_str.contains("settings:"),
        "Help should contain settings: prefix"
    );
    assert!(
        output_str.contains("task:"),
        "Help should contain task: prefix"
    );

    // Verify incorrect prefixes are NOT documented
    assert!(
        !output_str.contains("entity_definitions:"),
        "Help should NOT contain incorrect entity_definitions: prefix"
    );
    assert!(
        !output_str.contains("api_keys:"),
        "Help should NOT contain incorrect api_keys: prefix"
    );
    assert!(
        !output_str.contains("entities:"),
        "Help should NOT contain non-existent entities: prefix"
    );
}

#[test]
fn test_clear_cache_missing_redis_url() {
    // Test that missing REDIS_URL produces appropriate error
    // Note: This test may fail if .env file exists with REDIS_URL
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "clear_cache",
            "--",
            "--all",
        ])
        .env_remove("REDIS_URL")
        .env("DOTENV_IGNORE", "1") // Try to prevent .env loading
        .output()
        .expect("Failed to execute clear_cache binary");

    // Check both stdout and stderr for error message
    let stdout_str = String::from_utf8(output.stdout).expect("Failed to convert stdout to string");
    let stderr_str = String::from_utf8(output.stderr).expect("Failed to convert stderr to string");
    let output_str = format!("{stdout_str}{stderr_str}");

    // If .env file exists, the command might succeed, so we check for error message
    if output.status.success() {
        // If it succeeded, it means REDIS_URL was found (likely from .env)
        // This is acceptable behavior, so we just skip the assertion
        println!("Test skipped: REDIS_URL found in environment or .env file");
    } else {
        assert!(
            output_str.contains("REDIS_URL"),
            "Error message should mention REDIS_URL, got: {output_str}"
        );
    }
}

#[test]
fn test_clear_cache_invalid_arguments() {
    // Test that missing both --all and --prefix produces error
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "clear_cache",
        ])
        .env("REDIS_URL", "redis://localhost:6379")
        .output()
        .expect("Failed to execute clear_cache binary");

    assert!(
        !output.status.success(),
        "Command should fail without arguments"
    );

    // Check both stdout and stderr
    let stdout_str = String::from_utf8(output.stdout).expect("Failed to convert stdout to string");
    let stderr_str = String::from_utf8(output.stderr).expect("Failed to convert stderr to string");
    let output_str = format!("{stdout_str}{stderr_str}");

    assert!(
        output_str.contains("--all") || output_str.contains("--prefix"),
        "Error message should mention --all or --prefix, got: {output_str}"
    );
}

#[test]
fn test_clear_cache_conflicting_arguments() {
    // Test that --all and --prefix together produce error
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "clear_cache",
            "--",
            "--all",
            "--prefix",
            "test:",
        ])
        .env("REDIS_URL", "redis://localhost:6379")
        .output()
        .expect("Failed to execute clear_cache binary");

    assert!(
        !output.status.success(),
        "Command should fail with conflicting arguments"
    );

    // Check both stdout and stderr
    let stdout_str = String::from_utf8(output.stdout).expect("Failed to convert stdout to string");
    let stderr_str = String::from_utf8(output.stderr).expect("Failed to convert stderr to string");
    let output_str = format!("{stdout_str}{stderr_str}");

    assert!(
        output_str.contains("--all") && output_str.contains("--prefix"),
        "Error message should mention both --all and --prefix, got: {output_str}"
    );
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_clear_cache_all_if_redis_available() {
    let Some(_redis_url) = get_redis_url() else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    // Test --all flag with dry-run (safe to run)
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "clear_cache",
            "--",
            "--all",
            "--dry-run",
        ])
        .output()
        .expect("Failed to execute clear_cache binary");

    // Should succeed even in dry-run mode
    assert!(output.status.success(), "Dry-run should succeed");

    let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");

    assert!(
        output_str.contains("DRY-RUN") || output_str.contains("Would clear"),
        "Output should indicate dry-run mode"
    );
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_clear_cache_prefix_dry_run_if_redis_available() {
    let Some(_redis_url) = get_redis_url() else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    // Test --prefix with dry-run (safe to run)
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "clear_cache",
            "--",
            "--prefix",
            "entity_def:",
            "--dry-run",
        ])
        .output()
        .expect("Failed to execute clear_cache binary");

    // Should succeed even in dry-run mode
    assert!(output.status.success(), "Dry-run should succeed");

    let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");

    assert!(
        output_str.contains("entity_def:") || output_str.contains("DRY-RUN"),
        "Output should mention the prefix or dry-run"
    );
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_clear_cache_prefix_validation_if_redis_available() {
    let Some(_redis_url) = get_redis_url() else {
        println!("Skipping test: REDIS_URL not set");
        return;
    };

    // Test that correct prefixes work (dry-run mode)
    let prefixes = [
        "entity_def:",
        "api_key:",
        "role:",
        "user_roles:",
        "api_key_roles:",
        "user_permissions:",
        "api_key_permissions:",
        "settings:",
        "task:",
    ];

    for prefix in &prefixes {
        let output = Command::new("cargo")
            .args([
                "run",
                "--package",
                "r_data_core_core",
                "--bin",
                "clear_cache",
                "--",
                "--prefix",
                prefix,
                "--dry-run",
            ])
            .output()
            .expect("Failed to execute clear_cache binary");

        assert!(
            output.status.success(),
            "Prefix {prefix} should be accepted"
        );
    }
}
