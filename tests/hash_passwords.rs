#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use std::process::Command;

#[test]
fn test_hash_password_binary() {
    // Test with a simple password
    let test_password = "testpassword123";

    // Run the binary
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "hash_password",
            "--",
            test_password,
        ])
        .output()
        .expect("Failed to execute hash_password binary");

    // Check that the command succeeded
    assert!(output.status.success(), "Binary should exit successfully");

    // Get the output as string
    let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");

    // Verify the output contains expected elements
    assert!(
        output_str.contains(&format!("Password: {test_password}")),
        "Output should contain the input password"
    );
    assert!(
        output_str.contains("Hash: "),
        "Output should contain a hash"
    );
    assert!(
        output_str.contains("SQL:"),
        "Output should contain SQL section"
    );
    assert!(
        output_str.contains("UPDATE admin_users SET password_hash = "),
        "Output should contain SQL UPDATE statement"
    );

    // Verify the hash format (should be Argon2id format)
    let lines: Vec<&str> = output_str.lines().collect();
    let hash_line = lines
        .iter()
        .find(|line| line.starts_with("Hash: "))
        .expect("Should find hash line");

    let hash = hash_line.strip_prefix("Hash: ").unwrap();

    // Verify it's a valid Argon2id hash format
    // Argon2id hashes start with "$argon2id$"
    assert!(
        hash.starts_with("$argon2id$"),
        "Hash should be in Argon2id format"
    );

    // Verify the hash contains the expected parts
    let parts: Vec<&str> = hash.split('$').collect();
    assert!(
        parts.len() >= 6,
        "Argon2id hash should have at least 6 parts"
    );
    assert_eq!(parts[1], "argon2id", "Hash should use Argon2id algorithm");
}

#[test]
fn test_hash_password_binary_no_args() {
    // Test with no arguments (should show usage and exit successfully)
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "hash_password",
        ])
        .output()
        .expect("Failed to execute hash_password binary");

    // Check that the command succeeded (binary handles no args gracefully)
    assert!(
        output.status.success(),
        "Binary should handle no arguments gracefully"
    );

    // Get the output as string
    let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");

    // Verify it shows usage information
    assert!(
        output_str.contains("Usage:"),
        "Output should contain usage information"
    );
}

#[test]
fn test_hash_password_binary_empty_password() {
    // Test with empty password
    let test_password = "";

    // Run the binary
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "hash_password",
            "--",
            test_password,
        ])
        .output()
        .expect("Failed to execute hash_password binary");

    // Check that the command succeeded (empty password is valid)
    assert!(
        output.status.success(),
        "Binary should handle empty password"
    );

    // Get the output as string
    let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");

    // Verify the output contains expected elements
    assert!(
        output_str.contains("Password: "),
        "Output should contain the input password"
    );
    assert!(
        output_str.contains("Hash: "),
        "Output should contain a hash"
    );

    // Verify the hash format
    let lines: Vec<&str> = output_str.lines().collect();
    let hash_line = lines
        .iter()
        .find(|line| line.starts_with("Hash: "))
        .expect("Should find hash line");

    let hash = hash_line.strip_prefix("Hash: ").unwrap();
    assert!(
        hash.starts_with("$argon2id$"),
        "Hash should be in Argon2id format"
    );
}

#[test]
fn test_hash_password_binary_special_characters() {
    // Test with special characters in password
    let test_password = "p@ssw0rd!@#$%^&*()_+-=[]{}|;':\",./<>?";

    // Run the binary
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "hash_password",
            "--",
            test_password,
        ])
        .output()
        .expect("Failed to execute hash_password binary");

    // Check that the command succeeded
    assert!(
        output.status.success(),
        "Binary should handle special characters"
    );

    // Get the output as string
    let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");

    // Verify the output contains expected elements
    assert!(
        output_str.contains(&format!("Password: {test_password}")),
        "Output should contain the input password"
    );
    assert!(
        output_str.contains("Hash: "),
        "Output should contain a hash"
    );

    // Verify the hash format
    let lines: Vec<&str> = output_str.lines().collect();
    let hash_line = lines
        .iter()
        .find(|line| line.starts_with("Hash: "))
        .expect("Should find hash line");

    let hash = hash_line.strip_prefix("Hash: ").unwrap();
    assert!(
        hash.starts_with("$argon2id$"),
        "Hash should be in Argon2id format"
    );
}

#[test]
fn test_hash_password_binary_sql_output() {
    // Test that SQL output is properly formatted
    let test_password = "password'with'quotes";

    // Run the binary
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "r_data_core_core",
            "--bin",
            "hash_password",
            "--",
            test_password,
        ])
        .output()
        .expect("Failed to execute hash_password binary");

    // Check that the command succeeded
    assert!(
        output.status.success(),
        "Binary should handle passwords with quotes"
    );

    // Get the output as string
    let output_str = String::from_utf8(output.stdout).expect("Failed to convert output to string");

    // Find the SQL line
    let sql_line = output_str
        .lines()
        .find(|line| line.contains("UPDATE admin_users SET password_hash = "))
        .expect("Should find SQL UPDATE statement");

    // Verify the SQL statement is complete and properly formatted
    assert!(
        sql_line.ends_with(';'),
        "SQL statement should end with semicolon"
    );

    // Verify the SQL contains the hash (should be properly escaped if needed)
    assert!(
        sql_line.contains("password_hash = '"),
        "SQL should contain password_hash assignment"
    );

    // Verify the SQL statement is valid (no obvious syntax errors)
    assert!(
        !sql_line.contains("''''"),
        "SQL should not have double-escaped quotes"
    );
}
