use super::OutboxRepository;
use r_data_core_core::{error::Error, error::Result};
use time::OffsetDateTime;

impl OutboxRepository {
    /// Requeue stuck processing rows that have been locked for too long.
    ///
    /// # Errors
    /// Returns an error if the update fails.
    pub async fn requeue_stale_processing(&self, stale_before: OffsetDateTime) -> Result<i64> {
        let result = sqlx::query(
            r"
            UPDATE outbox_messages
            SET status = 'retry',
                available_at = NOW(),
                locked_at = NULL,
                locked_by = NULL,
                last_error = COALESCE(last_error, 'stale processing lease expired')
            WHERE status = 'processing'
              AND locked_at IS NOT NULL
              AND locked_at < $1
            ",
        )
        .bind(stale_before)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(i64::try_from(result.rows_affected()).unwrap_or(0))
    }

    /// Delete terminal outbox rows older than the configured cutoff.
    ///
    /// # Errors
    /// Returns an error if the delete fails.
    pub async fn purge_terminal_older_than(&self, processed_before: OffsetDateTime) -> Result<i64> {
        let result = sqlx::query(
            r"
            DELETE FROM outbox_messages
            WHERE processed_at IS NOT NULL
              AND processed_at < $1
              AND status IN ('delivered', 'dead_letter')
            ",
        )
        .bind(processed_before)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;
        Ok(i64::try_from(result.rows_affected()).unwrap_or(0))
    }
}
