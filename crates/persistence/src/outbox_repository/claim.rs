use super::{OutboxMessageRecord, OutboxRepository};
use r_data_core_core::{error::Error, error::Result};
use time::OffsetDateTime;

impl OutboxRepository {
    /// Claim due outbox messages for processing.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn claim_due(&self, limit: i64, worker_id: &str) -> Result<Vec<OutboxMessageRecord>> {
        let mut tx = self.pool.begin().await.map_err(Error::Database)?;
        let messages = sqlx::query_as::<_, OutboxMessageRecord>(
            r"
            WITH claimed AS (
                SELECT uuid
                FROM outbox_messages
                WHERE status IN ('pending', 'retry')
                  AND available_at <= NOW()
                ORDER BY available_at ASC, created_at ASC
                LIMIT $1
                FOR UPDATE SKIP LOCKED
            )
            UPDATE outbox_messages AS o
            SET status = 'processing',
                locked_at = NOW(),
                locked_by = $2
            FROM claimed
            WHERE o.uuid = claimed.uuid
            RETURNING o.uuid,
                      o.topic,
                      o.kind,
                      o.aggregate_type,
                      o.aggregate_id,
                      o.payload,
                      o.headers,
                      o.status,
                      o.attempt_count,
                      o.available_at,
                      o.locked_at,
                      o.locked_by,
                      o.last_error,
                      o.idempotency_key,
                      o.created_at,
                      o.processed_at
            ",
        )
        .bind(limit)
        .bind(worker_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(Error::Database)?;
        tx.commit().await.map_err(Error::Database)?;

        Ok(messages)
    }

    /// Return the earliest time at which a pending or retry outbox row becomes available.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn next_available_at(&self) -> Result<Option<OffsetDateTime>> {
        let next_available_at: Option<OffsetDateTime> = sqlx::query_scalar(
            r"
            SELECT MIN(available_at)
            FROM outbox_messages
            WHERE status IN ('pending', 'retry')
            ",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(next_available_at)
    }
}
