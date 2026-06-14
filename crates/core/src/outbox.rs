#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Outbox message topic for workflow fetch dispatches.
pub const WORKFLOW_FETCH_TOPIC: &str = "workflow.fetch.enqueue";

/// Outbox message kind for workflow fetch dispatches.
pub const WORKFLOW_FETCH_ENQUEUE_KIND: &str = "redis.fetch";

/// Outbox message topic for workflow push deliveries.
pub const WORKFLOW_PUSH_TOPIC: &str = "workflow.push.enqueue";

/// Outbox message kind for workflow push deliveries.
pub const WORKFLOW_PUSH_ENQUEUE_KIND: &str = "http.uri";

/// `PostgreSQL` notification channel used to wake the workflow outbox worker.
pub const WORKFLOW_OUTBOX_NOTIFY_CHANNEL: &str = "workflow_outbox_available";

/// Lifecycle state for outbox messages. Backed by the Postgres `outbox_status`
/// enum; `snake_case` to match the other status enums in the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "outbox_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OutboxStatus {
    Pending,
    Processing,
    Delivered,
    Retry,
    DeadLetter,
}

impl OutboxStatus {
    /// Return the database representation of the status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Delivered => "delivered",
            Self::Retry => "retry",
            Self::DeadLetter => "dead_letter",
        }
    }
}

impl std::fmt::Display for OutboxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for OutboxStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "delivered" => Ok(Self::Delivered),
            "retry" => Ok(Self::Retry),
            "dead_letter" => Ok(Self::DeadLetter),
            other => Err(format!("Invalid outbox status: {other}")),
        }
    }
}

/// Lightweight description of an outbox message.
///
/// The persistence layer uses this structure for claim/retry bookkeeping and the
/// worker uses it as the unit of dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxMessage {
    pub uuid: Uuid,
    pub topic: String,
    pub kind: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub payload: serde_json::Value,
    pub headers: serde_json::Value,
    pub status: OutboxStatus,
    pub attempt_count: i32,
    pub available_at: OffsetDateTime,
    pub locked_at: Option<OffsetDateTime>,
    pub locked_by: Option<String>,
    pub last_error: Option<String>,
    pub idempotency_key: String,
    pub created_at: OffsetDateTime,
    pub processed_at: Option<OffsetDateTime>,
}
