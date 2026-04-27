use super::OutboxRepository;
use r_data_core_core::{error::Error, error::Result};
use time::OffsetDateTime;
use uuid::Uuid;

impl OutboxRepository {
    /// Mark an outbox message as delivered.
    ///
    /// # Errors
    /// Returns an error if the update fails.
    pub async fn mark_delivered(&self, uuid: Uuid, locked_by: Option<&str>) -> Result<()> {
        let query = locked_by.map_or_else(
            || {
                sqlx::query(
                    r"
                    UPDATE outbox_messages
                    SET status = 'delivered',
                        processed_at = NOW(),
                        locked_at = NULL,
                        locked_by = NULL,
                        last_error = NULL
                    WHERE uuid = $1
                      AND status = 'pending'
                    ",
                )
                .bind(uuid)
            },
            |locked_by| {
                sqlx::query(
                    r"
                    UPDATE outbox_messages
                    SET status = 'delivered',
                        processed_at = NOW(),
                        locked_at = NULL,
                        locked_by = NULL,
                        last_error = NULL
                    WHERE uuid = $1
                      AND status = 'processing'
                      AND locked_by = $2
                    ",
                )
                .bind(uuid)
                .bind(locked_by)
            },
        );

        let result = query.execute(&self.pool).await.map_err(Error::Database)?;
        if result.rows_affected() == 0 {
            return Err(Error::Unknown(format!(
                "Outbox transition to delivered affected no rows for {uuid}",
            )));
        }
        Ok(())
    }

    /// Mark an outbox message for retry.
    ///
    /// # Errors
    /// Returns an error if the update fails.
    pub async fn mark_retry(
        &self,
        uuid: Uuid,
        last_error: &str,
        available_at: OffsetDateTime,
        locked_by: Option<&str>,
    ) -> Result<()> {
        let query = locked_by.map_or_else(
            || {
                sqlx::query(
                    r"
                    UPDATE outbox_messages
                    SET status = 'retry',
                        attempt_count = attempt_count,
                        available_at = $2,
                        locked_at = NULL,
                        locked_by = NULL,
                        last_error = $3
                    WHERE uuid = $1
                      AND status = 'pending'
                    ",
                )
                .bind(uuid)
                .bind(available_at)
                .bind(last_error)
            },
            |locked_by| {
                sqlx::query(
                    r"
                    UPDATE outbox_messages
                    SET status = 'retry',
                        attempt_count = attempt_count + 1,
                        available_at = $2,
                        locked_at = NULL,
                        locked_by = NULL,
                        last_error = $3
                    WHERE uuid = $1
                      AND status = 'processing'
                      AND locked_by = $4
                    ",
                )
                .bind(uuid)
                .bind(available_at)
                .bind(last_error)
                .bind(locked_by)
            },
        );

        let result = query.execute(&self.pool).await.map_err(Error::Database)?;
        if result.rows_affected() == 0 {
            return Err(Error::Unknown(format!(
                "Outbox transition to retry affected no rows for {uuid}",
            )));
        }
        Ok(())
    }

    /// Mark an outbox message as dead-lettered.
    ///
    /// # Errors
    /// Returns an error if the update fails.
    pub async fn mark_dead_letter(
        &self,
        uuid: Uuid,
        last_error: &str,
        locked_by: Option<&str>,
    ) -> Result<()> {
        let query = locked_by.map_or_else(
            || {
                sqlx::query(
                    r"
                    UPDATE outbox_messages
                    SET status = 'dead_letter',
                        attempt_count = attempt_count,
                        processed_at = NOW(),
                        locked_at = NULL,
                        locked_by = NULL,
                        last_error = $2
                    WHERE uuid = $1
                      AND status = 'pending'
                    ",
                )
                .bind(uuid)
                .bind(last_error)
            },
            |locked_by| {
                sqlx::query(
                    r"
                    UPDATE outbox_messages
                    SET status = 'dead_letter',
                        attempt_count = attempt_count + 1,
                        processed_at = NOW(),
                        locked_at = NULL,
                        locked_by = NULL,
                        last_error = $2
                    WHERE uuid = $1
                      AND status = 'processing'
                      AND locked_by = $3
                    ",
                )
                .bind(uuid)
                .bind(last_error)
                .bind(locked_by)
            },
        );

        let result = query.execute(&self.pool).await.map_err(Error::Database)?;
        if result.rows_affected() == 0 {
            return Err(Error::Unknown(format!(
                "Outbox transition to dead_letter affected no rows for {uuid}",
            )));
        }
        Ok(())
    }
}
