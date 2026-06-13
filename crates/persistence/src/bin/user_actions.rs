#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Console binary for admin-operator user management.
//!
//! Allows an operator to lock, unlock, activate, deactivate, or reset the
//! password of an admin user directly from the command line — useful when the
//! operator has locked themselves out of the web UI.
//!
//! # Usage
//!
//! ```bash
//! user_actions --username <name> --action <lock|unlock|activate|deactivate|password-reset> [--password <new>]
//! ```
//!
//! # Environment Variables
//!
//! - `DATABASE_URL` — Postgres connection string (required)
//!
//! # Exit Codes
//!
//! - 0: Success
//! - 1: Error (missing args, unknown user, database failure, etc.)

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use dotenvy::dotenv;
use r_data_core_core::admin_user::UserStatus;
use r_data_core_persistence::admin_user_repository_trait::AdminUserRepositoryTrait as _;
use r_data_core_persistence::AdminUserRepository;
use sqlx::postgres::PgPoolOptions;

/// Supported actions for the `user_actions` binary.
#[derive(Clone, Copy)]
enum Action {
    Lock,
    Unlock,
    Activate,
    Deactivate,
    PasswordReset,
}

impl Action {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "lock" => Some(Self::Lock),
            "unlock" => Some(Self::Unlock),
            "activate" => Some(Self::Activate),
            "deactivate" => Some(Self::Deactivate),
            "password-reset" => Some(Self::PasswordReset),
            _ => None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Lock => "locked",
            Self::Unlock => "unlocked",
            Self::Activate => "activated",
            Self::Deactivate => "deactivated",
            Self::PasswordReset => "password reset",
        }
    }
}

/// Parsed CLI arguments.
struct Args {
    username: String,
    action: Action,
    password: Option<String>,
}

fn print_usage() {
    eprintln!("USAGE:");
    eprintln!(
        "    user_actions --username <name> \
         --action <lock|unlock|activate|deactivate|password-reset> \
         [--password <new>]"
    );
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --username <name>    Admin username to operate on (required)");
    eprintln!(
        "    --action   <action>  One of: lock, unlock, activate, deactivate, password-reset (required)"
    );
    eprintln!("    --password <new>     New plain-text password (required for password-reset)");
    eprintln!();
    eprintln!("ENVIRONMENT:");
    eprintln!("    DATABASE_URL         PostgreSQL connection string (required)");
}

/// Parse `--key value` pairs from argv.
///
/// Returns `None` (after printing an error to stderr) if a required argument
/// is absent or an unrecognised action is supplied.
fn parse_args() -> Option<Args> {
    let raw: Vec<String> = env::args().collect();
    let mut username: Option<String> = None;
    let mut action_str: Option<String> = None;
    let mut password: Option<String> = None;

    let mut i = 1usize;
    while i < raw.len() {
        match raw[i].as_str() {
            "--username" => {
                i += 1;
                username = raw.get(i).cloned();
            }
            "--action" => {
                i += 1;
                action_str = raw.get(i).cloned();
            }
            "--password" => {
                i += 1;
                password = raw.get(i).cloned();
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                eprintln!("Error: unknown option '{other}'");
                print_usage();
                return None;
            }
        }
        i += 1;
    }

    let Some(username) = username else {
        eprintln!("Error: --username is required");
        print_usage();
        return None;
    };

    let Some(action_str) = action_str else {
        eprintln!("Error: --action is required");
        print_usage();
        return None;
    };

    let Some(action) = Action::parse(&action_str) else {
        eprintln!(
            "Error: unknown action '{action_str}'. \
             Valid: lock, unlock, activate, deactivate, password-reset"
        );
        print_usage();
        return None;
    };

    if matches!(action, Action::PasswordReset) && password.is_none() {
        eprintln!("Error: --password is required for the password-reset action");
        print_usage();
        return None;
    }

    Some(Args {
        username,
        action,
        password,
    })
}

#[tokio::main]
async fn main() -> ExitCode {
    dotenv().ok();

    let Some(args) = parse_args() else {
        return ExitCode::FAILURE;
    };

    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("Error: DATABASE_URL environment variable is not set");
        eprintln!();
        eprintln!("Set it in your environment or in a .env file:");
        eprintln!("  DATABASE_URL=postgres://user:password@host:5432/database");
        return ExitCode::FAILURE;
    };

    let pool = match PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: Failed to connect to database: {e}");
            return ExitCode::FAILURE;
        }
    };

    let repo = AdminUserRepository::new(Arc::new(pool));

    let user = match repo.find_by_username_or_email(&args.username).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            eprintln!("Error: user '{}' not found", args.username);
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("Error looking up user '{}': {e}", args.username);
            return ExitCode::FAILURE;
        }
    };

    let action = args.action;

    let result = match action {
        Action::Lock => {
            repo.update_lockout_state(&user.uuid, &UserStatus::Locked, 5)
                .await
        }
        Action::Unlock => {
            repo.update_lockout_state(&user.uuid, &UserStatus::Active, 0)
                .await
        }
        Action::Activate => repo.set_active(&user.uuid, true).await,
        Action::Deactivate => repo.set_active(&user.uuid, false).await,
        Action::PasswordReset => {
            // parse_args guarantees password is Some for this variant.
            let plain = args.password.unwrap_or_default();
            match r_data_core_core::crypto::hash_password_argon2(&plain) {
                Ok(hash) => repo.reset_password(&user.uuid, &hash).await,
                Err(e) => {
                    eprintln!("Error hashing password: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
    };

    match result {
        Ok(()) => {
            println!("user '{}' {}", args.username, action.label());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
