#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Migration service for managing database schema migrations.
//!
//! This service provides functionality for running and checking database migrations.

use sqlx::PgPool;

use crate::core::error::{Error, Result};

/// Information about an applied migration
#[derive(Debug, Clone)]
pub struct AppliedMigration {
    /// Migration version number
    pub version: i64,
    /// Migration description
    pub description: String,
}

/// Result of a migration status check
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    /// Whether the migrations table exists
    pub table_exists: bool,
    /// List of applied migrations
    pub applied_migrations: Vec<AppliedMigration>,
}

impl MigrationStatus {
    /// Get the total number of applied migrations
    #[must_use]
    pub const fn applied_count(&self) -> usize {
        self.applied_migrations.len()
    }

    /// Check if any migrations have been applied
    #[must_use]
    pub const fn has_migrations(&self) -> bool {
        self.table_exists && !self.applied_migrations.is_empty()
    }
}

/// Migration service for database schema management
pub struct MigrationService {
    pool: PgPool,
}

impl MigrationService {
    /// Create a new migration service
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run all pending migrations
    ///
    /// # Errors
    /// Returns an error if migrations fail to run
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("already exists") {
                    // Some objects already exist, this is often fine in idempotent scenarios
                    return Error::Database(sqlx::Error::Configuration(
                        "Some migration objects already exist".into(),
                    ));
                }
                Error::Database(sqlx::Error::Configuration(
                    format!("Migration failed: {e}").into(),
                ))
            })?;
        Ok(())
    }

    /// Check migration status without running migrations
    ///
    /// # Errors
    /// Returns an error if the status check fails
    pub async fn check_status(&self) -> Result<MigrationStatus> {
        // Check if _sqlx_migrations table exists
        let table_exists: bool = sqlx::query_scalar(
            r"SELECT EXISTS (
                SELECT FROM information_schema.tables
                WHERE table_schema = 'public'
                AND table_name = '_sqlx_migrations'
            )",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        if !table_exists {
            return Ok(MigrationStatus {
                table_exists: false,
                applied_migrations: Vec::new(),
            });
        }

        // Get applied migrations
        let rows: Vec<(i64, String)> =
            sqlx::query_as(r"SELECT version, description FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&self.pool)
                .await
                .map_err(Error::Database)?;

        let applied_migrations = rows
            .into_iter()
            .map(|(version, description)| AppliedMigration {
                version,
                description,
            })
            .collect();

        Ok(MigrationStatus {
            table_exists: true,
            applied_migrations,
        })
    }

    /// Get reference to the underlying pool
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_status_no_migrations() {
        let status = MigrationStatus {
            table_exists: false,
            applied_migrations: Vec::new(),
        };

        assert!(!status.has_migrations());
        assert_eq!(status.applied_count(), 0);
    }

    #[test]
    fn test_migration_status_with_migrations() {
        let status = MigrationStatus {
            table_exists: true,
            applied_migrations: vec![
                AppliedMigration {
                    version: 1,
                    description: "Initial".to_string(),
                },
                AppliedMigration {
                    version: 2,
                    description: "Add users".to_string(),
                },
            ],
        };

        assert!(status.has_migrations());
        assert_eq!(status.applied_count(), 2);
    }

    #[test]
    fn test_migration_status_table_exists_no_migrations() {
        let status = MigrationStatus {
            table_exists: true,
            applied_migrations: Vec::new(),
        };

        // Table exists but no migrations applied
        assert!(!status.has_migrations());
        assert_eq!(status.applied_count(), 0);
    }
}
