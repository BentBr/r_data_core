#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::outbox::{OutboxMessage, OutboxStatus, WORKFLOW_OUTBOX_NOTIFY_CHANNEL};
use r_data_core_core::{error::Error, error::Result};
use sqlx::{FromRow, PgPool, Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

/// Repository for workflow outbox messages.
#[derive(Clone)]
pub struct OutboxRepository {
    pool: PgPool,
}

struct OutboxInsertMessage<'a> {
    topic: &'a str,
    kind: &'a str,
    aggregate_type: &'a str,
    aggregate_id: String,
    payload: serde_json::Value,
    headers: serde_json::Value,
    idempotency_key: String,
}

impl OutboxRepository {
    /// Create a new outbox repository.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a workflow fetch dispatch message in the outbox.
    ///
    /// # Errors
    /// Returns an error if the insert fails.
    pub async fn insert_workflow_fetch_enqueue(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> Result<Uuid> {
        let payload = serde_json::json!({
            "workflow_id": workflow_uuid,
            "trigger_id": run_uuid,
        });
        let headers = serde_json::json!({
            "workflow_id": workflow_uuid,
            "run_uuid": run_uuid,
            "topic": r_data_core_core::outbox::WORKFLOW_FETCH_TOPIC,
        });
        let idempotency_key = format!("workflow.fetch.enqueue:{run_uuid}");

        self.insert_message(OutboxInsertMessage {
            topic: r_data_core_core::outbox::WORKFLOW_FETCH_TOPIC,
            kind: r_data_core_core::outbox::WORKFLOW_FETCH_ENQUEUE_KIND,
            aggregate_type: "workflow_run",
            aggregate_id: run_uuid.to_string(),
            payload,
            headers,
            idempotency_key,
        })
        .await
    }

    /// Insert a workflow push delivery message in the outbox.
    ///
    /// # Errors
    /// Returns an error if the insert fails.
    pub async fn insert_workflow_push_enqueue(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        item_uuid: Uuid,
        payload: serde_json::Value,
        headers: serde_json::Value,
        destination_fingerprint: &str,
    ) -> Result<Uuid> {
        let idempotency_key =
            format!("workflow.push.enqueue:{workflow_uuid}:{run_uuid}:{item_uuid}:{destination_fingerprint}");

        self.insert_message(OutboxInsertMessage {
            topic: r_data_core_core::outbox::WORKFLOW_PUSH_TOPIC,
            kind: r_data_core_core::outbox::WORKFLOW_PUSH_ENQUEUE_KIND,
            aggregate_type: "workflow_item",
            aggregate_id: item_uuid.to_string(),
            payload,
            headers,
            idempotency_key,
        })
        .await
    }

    /// Insert a workflow fetch dispatch message in the outbox inside an existing transaction.
    ///
    /// # Errors
    /// Returns an error if the insert fails.
    pub async fn insert_workflow_fetch_enqueue_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> Result<Uuid> {
        let payload = serde_json::json!({
            "workflow_id": workflow_uuid,
            "trigger_id": run_uuid,
        });
        let headers = serde_json::json!({
            "workflow_id": workflow_uuid,
            "run_uuid": run_uuid,
            "topic": r_data_core_core::outbox::WORKFLOW_FETCH_TOPIC,
        });
        let idempotency_key = format!("workflow.fetch.enqueue:{run_uuid}");
        let row = sqlx::query(
            r"
            INSERT INTO outbox_messages (
                topic,
                kind,
                aggregate_type,
                aggregate_id,
                payload,
                headers,
                status,
                attempt_count,
                available_at,
                idempotency_key
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'pending', 0, NOW(), $7)
            ON CONFLICT (idempotency_key) DO UPDATE
                SET payload = EXCLUDED.payload,
                    headers = EXCLUDED.headers,
                    status = 'pending',
                    attempt_count = 0,
                    available_at = NOW(),
                    last_error = NULL,
                    locked_at = NULL,
                    locked_by = NULL,
                    processed_at = NULL
            RETURNING uuid
            ",
        )
        .bind(r_data_core_core::outbox::WORKFLOW_FETCH_TOPIC)
        .bind(r_data_core_core::outbox::WORKFLOW_FETCH_ENQUEUE_KIND)
        .bind("workflow_run")
        .bind(run_uuid.to_string())
        .bind(payload)
        .bind(headers)
        .bind(idempotency_key)
        .fetch_one(&mut **tx)
        .await
        .map_err(Error::Database)?;

        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(WORKFLOW_OUTBOX_NOTIFY_CHANNEL)
            .bind("workflow outbox available")
            .execute(&mut **tx)
            .await
            .map_err(Error::Database)?;
        Ok(row.try_get("uuid")?)
    }

    async fn insert_message(&self, message: OutboxInsertMessage<'_>) -> Result<Uuid> {
        let row = sqlx::query(
            r"
            INSERT INTO outbox_messages (
                topic,
                kind,
                aggregate_type,
                aggregate_id,
                payload,
                headers,
                status,
                attempt_count,
                available_at,
                idempotency_key
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'pending', 0, NOW(), $7)
            ON CONFLICT (idempotency_key) DO UPDATE
                SET payload = EXCLUDED.payload,
                    headers = EXCLUDED.headers,
                    status = 'pending',
                    attempt_count = 0,
                    available_at = NOW(),
                    last_error = NULL,
                    locked_at = NULL,
                    locked_by = NULL,
                    processed_at = NULL
            RETURNING uuid
            ",
        )
        .bind(message.topic)
        .bind(message.kind)
        .bind(message.aggregate_type)
        .bind(message.aggregate_id)
        .bind(message.payload)
        .bind(message.headers)
        .bind(message.idempotency_key)
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(WORKFLOW_OUTBOX_NOTIFY_CHANNEL)
            .bind("workflow outbox available")
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;
        Ok(row.try_get("uuid")?)
    }

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

        query.execute(&self.pool).await.map_err(Error::Database)?;
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

        query.execute(&self.pool).await.map_err(Error::Database)?;
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

        query.execute(&self.pool).await.map_err(Error::Database)?;
        Ok(())
    }

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

/// Raw outbox row mapped from `PostgreSQL`.
#[derive(Debug, Clone, FromRow)]
pub struct OutboxMessageRecord {
    pub uuid: Uuid,
    pub topic: String,
    pub kind: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub payload: serde_json::Value,
    pub headers: serde_json::Value,
    pub status: String,
    pub attempt_count: i32,
    pub available_at: OffsetDateTime,
    pub locked_at: Option<OffsetDateTime>,
    pub locked_by: Option<String>,
    pub last_error: Option<String>,
    pub idempotency_key: String,
    pub created_at: OffsetDateTime,
    pub processed_at: Option<OffsetDateTime>,
}

impl OutboxMessageRecord {
    /// Convert the row into the core message representation.
    #[must_use]
    pub fn into_message(self) -> OutboxMessage {
        let status = self
            .status
            .parse::<OutboxStatus>()
            .unwrap_or(OutboxStatus::Pending);

        OutboxMessage {
            uuid: self.uuid,
            topic: self.topic,
            kind: self.kind,
            aggregate_type: self.aggregate_type,
            aggregate_id: self.aggregate_id,
            payload: self.payload,
            headers: self.headers,
            status,
            attempt_count: self.attempt_count,
            available_at: self.available_at,
            locked_at: self.locked_at,
            locked_by: self.locked_by,
            last_error: self.last_error,
            idempotency_key: self.idempotency_key,
            created_at: self.created_at,
            processed_at: self.processed_at,
        }
    }
}
