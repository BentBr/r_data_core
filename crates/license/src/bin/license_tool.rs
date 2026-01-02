#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use clap::{Parser, Subcommand};
use r_data_core_core::config::load_license_config;
use r_data_core_license::{LicenseCheckState, LicenseToolService};
use std::io::{self, Read};
use std::path::Path;

#[derive(Parser)]
#[command(name = "license_tool")]
#[command(about = "Create and verify RDataCore license keys")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new license key
    Create {
        /// Company name
        #[arg(long, required = true)]
        company: String,
        /// License type (community, education, company I, company II, company III, Enterprise, society I, society II, society III)
        #[arg(long, required = true)]
        license_type: String,
        /// Path to private key file (RSA PEM format)
        #[arg(long, required = true)]
        private_key_file: String,
        /// Output file path (optional, defaults to stdout)
        #[arg(long)]
        output: Option<String>,
        /// Expiration in days (optional, defaults to no expiration)
        #[arg(long)]
        expires_days: Option<u64>,
    },
    /// Verify a license key
    Verify {
        /// License key JWT token (or read from stdin if not provided)
        #[arg(long)]
        license_key: Option<String>,
        /// Path to license key file (alternative to --license-key)
        #[arg(long)]
        license_key_file: Option<String>,
        /// Path to public key file (RSA PEM format)
        #[arg(long, required = true)]
        public_key_file: String,
    },
    /// Check license key against the verification API (uses `LICENSE_KEY` from environment)
    Check,
}

fn handle_create(
    company: &str,
    license_type: &str,
    private_key_file: &str,
    output: Option<&str>,
    expires_days: Option<u64>,
) {
    let result =
        LicenseToolService::create_license(company, license_type, private_key_file, expires_days)
            .unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

    // Output license details
    println!("License Key Created:");
    println!("Company: {}", result.company);
    println!("Type: {}", result.license_type);
    println!("License ID: {}", result.license_id);
    println!(
        "Issued: {}",
        result
            .issued_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "Invalid date".to_string())
    );
    println!("Version: {}", result.version);
    if let Some(expires) = &result.expires {
        println!(
            "Expires: {}",
            expires
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "Invalid date".to_string())
        );
    }
    println!();
    println!("License Key:");
    println!("{}", result.token);

    // Write to file if specified
    if let Some(output_path) = output {
        let path = Path::new(output_path);
        if let Err(e) = LicenseToolService::write_license_to_file(&result.token, path) {
            eprintln!("Error: Failed to write to output file: {e}");
            std::process::exit(1);
        }
        println!();
        println!("License key written to: {output_path}");
    }
}

fn read_license_key(license_key: Option<&str>, license_key_file: Option<&str>) -> String {
    license_key.map_or_else(
        || {
            license_key_file.map_or_else(
                || {
                    // Read from stdin
                    let mut buffer = String::new();
                    io::stdin().read_to_string(&mut buffer).unwrap_or_else(|e| {
                        eprintln!("Error: Failed to read from stdin: {e}");
                        std::process::exit(1);
                    });
                    buffer.trim().to_string()
                },
                |key_file| {
                    std::fs::read_to_string(key_file).unwrap_or_else(|e| {
                        eprintln!("Error: Failed to read license key file: {e}");
                        std::process::exit(1);
                    })
                },
            )
        },
        str::to_string,
    )
}

fn handle_verify(license_key: Option<&str>, license_key_file: Option<&str>, public_key_file: &str) {
    let license_key_str = read_license_key(license_key, license_key_file);

    let result = LicenseToolService::verify_license(&license_key_str, public_key_file)
        .unwrap_or_else(|e| {
            eprintln!("Error: {e}");
            std::process::exit(1);
        });

    println!("License Verification:");
    if result.is_valid {
        println!("Status: VALID");
        println!("Company: {}", result.company);
        println!("Type: {}", result.license_type);
        println!("License ID: {}", result.license_id);
        println!(
            "Issued: {}",
            result
                .issued_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "Invalid date".to_string())
        );
        println!("Version: {}", result.version);
        if let Some(expires) = &result.expires {
            println!(
                "Expires: {}",
                expires
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| "Invalid date".to_string())
            );
        } else {
            println!("Expires: Never");
        }
    } else {
        println!("Status: INVALID");
        if let Some(error) = &result.error {
            println!("Error: {error}");
        }
        std::process::exit(1);
    }
}

fn handle_check() {
    // Load license config from environment (uses global config loader)
    let config = load_license_config().unwrap_or_else(|e| {
        eprintln!("Error: Failed to load license configuration: {e}");
        std::process::exit(1);
    });

    // Check if license key is provided
    let license_key = match &config.license_key {
        Some(key) if !key.is_empty() => key,
        _ => {
            eprintln!("Error: LICENSE_KEY environment variable is not set or empty");
            std::process::exit(1);
        }
    };

    // Run async check using the same API call logic as maintenance task
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!("Error: Failed to create async runtime: {e}");
            std::process::exit(1);
        });

    let result = rt
        .block_on(LicenseToolService::check_license(
            license_key,
            &config.verification_url,
        ))
        .unwrap_or_else(|e| {
            eprintln!("Error: Failed to check license: {e}");
            std::process::exit(1);
        });

    // Display results (same format as maintenance task would log)
    println!("License API Check:");
    println!("Status: {:?}", result.state);
    if let Some(company) = &result.company {
        println!("Company: {company}");
    }
    if let Some(license_type) = &result.license_type {
        println!("Type: {license_type}");
    }
    if let Some(license_id) = &result.license_id {
        println!("License ID: {license_id}");
    }
    if let Some(issued_at) = &result.issued_at {
        println!(
            "Issued: {}",
            issued_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "Invalid date".to_string())
        );
    }
    if let Some(version) = &result.version {
        println!("Version: {version}");
    }
    println!(
        "Verified at: {}",
        result
            .verified_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "Invalid date".to_string())
    );
    if let Some(error_message) = &result.error_message {
        println!("Error: {error_message}");
    }

    // Exit with error code if license is not valid
    if result.state != LicenseCheckState::Valid {
        std::process::exit(1);
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create {
            company,
            license_type,
            private_key_file,
            output,
            expires_days,
        } => handle_create(
            &company,
            &license_type,
            &private_key_file,
            output.as_deref(),
            expires_days,
        ),
        Commands::Verify {
            license_key,
            license_key_file,
            public_key_file,
        } => handle_verify(
            license_key.as_deref(),
            license_key_file.as_deref(),
            &public_key_file,
        ),
        Commands::Check => handle_check(),
    }
}
