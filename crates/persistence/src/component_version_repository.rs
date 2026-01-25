#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Repository for component version tracking.
//!
//! This module provides functionality to track and query versions
//! of distributed components like worker and maintenance.

use sqlx::PgPool;
use time::OffsetDateTime;

/// A component version record
#[derive(Debug, Clone)]
pub struct ComponentVersion {
    /// Name of the component (e.g., "worker", "maintenance")
    pub component_name: String,
    /// Semantic version string
    pub version: String,
    /// Last time the component reported its version
    pub last_seen_at: OffsetDateTime,
}

/// Repository for managing component versions
#[derive(Clone)]
pub struct ComponentVersionRepository {
    pool: PgPool,
}

impl ComponentVersionRepository {
    /// Create a new repository
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert a component version (insert or update)
    ///
    /// This is called by components on startup to register their version.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn upsert(&self, component_name: &str, version: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO component_versions (component_name, version, last_seen_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (component_name)
            DO UPDATE SET version = $2, last_seen_at = NOW()
            ",
        )
        .bind(component_name)
        .bind(version)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all component versions
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn get_all(&self) -> Result<Vec<ComponentVersion>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, OffsetDateTime)>(
            r"
            SELECT component_name, version, last_seen_at
            FROM component_versions
            ORDER BY component_name
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(component_name, version, last_seen_at)| ComponentVersion {
                component_name,
                version,
                last_seen_at,
            })
            .collect())
    }

    /// Get a specific component version
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn get(&self, component_name: &str) -> Result<Option<ComponentVersion>, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, String, OffsetDateTime)>(
            r"
            SELECT component_name, version, last_seen_at
            FROM component_versions
            WHERE component_name = $1
            ",
        )
        .bind(component_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(
            row.map(|(component_name, version, last_seen_at)| ComponentVersion {
                component_name,
                version,
                last_seen_at,
            }),
        )
    }
}
