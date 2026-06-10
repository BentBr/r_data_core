use super::types::OutboxInsertMessage;
use super::OutboxRepository;
use r_data_core_core::outbox::{
    WORKFLOW_FETCH_ENQUEUE_KIND, WORKFLOW_FETCH_TOPIC, WORKFLOW_OUTBOX_NOTIFY_CHANNEL,
    WORKFLOW_PUSH_ENQUEUE_KIND, WORKFLOW_PUSH_TOPIC,
};
use r_data_core_core::{error::Error, error::Result};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

impl OutboxRepository {
    /// Create a new outbox repository.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Ensure the required outbox table exists before outbox workers/services start.
    ///
    /// # Errors
    /// Returns an error when `outbox_messages` is missing.
    pub async fn ensure_table_exists(pool: &PgPool) -> Result<()> {
        let relation: Option<String> =
            sqlx::query_scalar("SELECT to_regclass('public.outbox_messages')::text")
                .fetch_one(pool)
                .await
                .map_err(Error::Database)?;
        if relation.is_none() {
            return Err(Error::Config(
                "OUTBOX_ENABLED=true but table 'outbox_messages' is missing. Run migrations first."
                    .to_string(),
            ));
        }
        Ok(())
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
            "topic": WORKFLOW_FETCH_TOPIC,
        });
        let idempotency_key = format!("workflow.fetch.enqueue:{run_uuid}");

        self.insert_message(OutboxInsertMessage {
            topic: WORKFLOW_FETCH_TOPIC,
            kind: WORKFLOW_FETCH_ENQUEUE_KIND,
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
        let idempotency_key = format!(
            "workflow.push.enqueue:{workflow_uuid}:{run_uuid}:{item_uuid}:{destination_fingerprint}"
        );

        self.insert_message(OutboxInsertMessage {
            topic: WORKFLOW_PUSH_TOPIC,
            kind: WORKFLOW_PUSH_ENQUEUE_KIND,
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
            "topic": WORKFLOW_FETCH_TOPIC,
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
        .bind(WORKFLOW_FETCH_TOPIC)
        .bind(WORKFLOW_FETCH_ENQUEUE_KIND)
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
}
