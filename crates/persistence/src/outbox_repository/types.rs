use r_data_core_core::outbox::{OutboxMessage, OutboxStatus};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

/// Repository for workflow outbox messages.
#[derive(Clone)]
pub struct OutboxRepository {
    pub(super) pool: PgPool,
}

pub(super) struct OutboxInsertMessage<'a> {
    pub(super) topic: &'a str,
    pub(super) kind: &'a str,
    pub(super) aggregate_type: &'a str,
    pub(super) aggregate_id: String,
    pub(super) payload: serde_json::Value,
    pub(super) headers: serde_json::Value,
    pub(super) idempotency_key: String,
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
