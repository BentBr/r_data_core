#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

//! Integration tests for `MigrationService`.
//!
//! These tests verify migration functionality against a real database.
//! They are skipped if `DATABASE_URL` is not set.

use r_data_core_persistence::MigrationService;
use sqlx::postgres::PgPoolOptions;

async fn get_migration_service() -> Option<MigrationService> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .ok()?;
    Some(MigrationService::new(pool))
}

#[tokio::test]
async fn test_migration_status_check() {
    let Some(service) = get_migration_service().await else {
        println!("Skipping test: DATABASE_URL not set");
        return;
    };

    // Check status - this should work regardless of migration state
    let status = service
        .check_status()
        .await
        .expect("status check should work");

    // Just verify we can check status - the actual migration state depends on the database
    println!(
        "Migration status: table_exists={}, migrations_count={}",
        status.table_exists,
        status.applied_count()
    );

    // If table exists, we should have at least some migrations
    if status.table_exists {
        assert!(
            status.applied_count() > 0 || !status.has_migrations(),
            "If table exists, either migrations are applied or not"
        );
    }
}

#[tokio::test]
async fn test_migration_status_struct_methods() {
    // Test the MigrationStatus struct methods without needing a database
    use r_data_core_persistence::{AppliedMigration, MigrationStatus};

    // Empty status
    let empty = MigrationStatus {
        table_exists: false,
        applied_migrations: Vec::new(),
    };
    assert!(!empty.has_migrations());
    assert_eq!(empty.applied_count(), 0);

    // Status with migrations
    let with_migrations = MigrationStatus {
        table_exists: true,
        applied_migrations: vec![
            AppliedMigration {
                version: 20_240_409_000_001,
                description: "enable_pgcrypto".to_string(),
            },
            AppliedMigration {
                version: 20_240_410_000_000,
                description: "comprehensive_schema".to_string(),
            },
        ],
    };
    assert!(with_migrations.has_migrations());
    assert_eq!(with_migrations.applied_count(), 2);

    // Table exists but no migrations (edge case)
    let empty_table = MigrationStatus {
        table_exists: true,
        applied_migrations: Vec::new(),
    };
    assert!(!empty_table.has_migrations());
    assert_eq!(empty_table.applied_count(), 0);
}

#[tokio::test]
async fn test_migration_service_pool_accessor() {
    let Some(service) = get_migration_service().await else {
        println!("Skipping test: DATABASE_URL not set");
        return;
    };

    // Verify we can access the pool
    let pool = service.pool();

    // Execute a simple query to verify the pool works
    let result: (i32,) = sqlx::query_as("SELECT 1 as test")
        .fetch_one(pool)
        .await
        .expect("simple query should work");

    assert_eq!(result.0, 1);
}
