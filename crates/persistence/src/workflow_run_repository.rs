#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::PgPool;

use crate::core::error::{Error, Result};

/// Repository for workflow run operations (pruning, etc.)
pub struct WorkflowRunRepository {
    pool: PgPool,
}

impl WorkflowRunRepository {
    /// Create a new workflow run repository
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // PgPool is not const
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Prune terminal workflow runs older than the specified number of days
    ///
    /// Runs with status `queued` or `running` are never pruned.
    /// Associated `workflow_run_logs` and `workflow_raw_items` are removed
    /// via `ON DELETE CASCADE`.
    ///
    /// # Arguments
    /// * `days` - Maximum age in days; runs older than this are deleted
    ///
    /// # Returns
    /// Number of rows deleted
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn prune_older_than_days(&self, days: i32) -> Result<u64> {
        let res = sqlx::query(
            "DELETE FROM workflow_runs
             WHERE queued_at < NOW() - make_interval(days => $1)
               AND status NOT IN ('queued', 'running')",
        )
        .bind(days)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(res.rows_affected())
    }

    /// Prune terminal workflow runs, keeping only the latest N per workflow
    ///
    /// Runs with status `queued` or `running` are never pruned.
    /// Associated `workflow_run_logs` and `workflow_raw_items` are removed
    /// via `ON DELETE CASCADE`.
    ///
    /// # Arguments
    /// * `keep` - Number of latest terminal runs to keep per workflow
    ///
    /// # Returns
    /// Number of rows deleted
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn prune_keep_latest_per_workflow(&self, keep: i32) -> Result<u64> {
        let res = sqlx::query(
            "DELETE FROM workflow_runs
             WHERE uuid IN (
                 SELECT uuid FROM (
                     SELECT uuid,
                            ROW_NUMBER() OVER (
                                PARTITION BY workflow_uuid
                                ORDER BY queued_at DESC
                            ) AS rn
                     FROM workflow_runs
                     WHERE status NOT IN ('queued', 'running')
                 ) ranked
                 WHERE rn > $1
             )",
        )
        .bind(keep)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(res.rows_affected())
    }
}
