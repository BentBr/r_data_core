#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_core::outbox::OutboxMessage;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::outbox_repository::{OutboxMessageRecord, OutboxRepository};

/// Repository contract for workflow outbox dispatch.
#[async_trait::async_trait]
pub trait OutboxRepositoryTrait: Send + Sync {
    async fn insert_workflow_fetch_enqueue(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> Result<Uuid>;

    #[allow(clippy::too_many_arguments)]
    async fn insert_workflow_push_enqueue(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        item_uuid: Uuid,
        payload: serde_json::Value,
        headers: serde_json::Value,
        destination_fingerprint: &str,
    ) -> Result<Uuid>;

    async fn claim_due(&self, limit: i64, worker_id: &str) -> Result<Vec<OutboxMessage>>;

    async fn next_available_at(&self) -> Result<Option<OffsetDateTime>>;

    async fn requeue_stale_processing(&self, stale_before: OffsetDateTime) -> Result<i64>;

    async fn mark_delivered(&self, uuid: Uuid, locked_by: Option<&str>) -> Result<()>;

    async fn mark_retry(
        &self,
        uuid: Uuid,
        last_error: &str,
        available_at: OffsetDateTime,
        locked_by: Option<&str>,
    ) -> Result<()>;

    async fn mark_dead_letter(
        &self,
        uuid: Uuid,
        last_error: &str,
        locked_by: Option<&str>,
    ) -> Result<()>;
}

#[async_trait::async_trait]
impl OutboxRepositoryTrait for OutboxRepository {
    async fn insert_workflow_fetch_enqueue(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> Result<Uuid> {
        Self::insert_workflow_fetch_enqueue(self, workflow_uuid, run_uuid).await
    }

    async fn insert_workflow_push_enqueue(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        item_uuid: Uuid,
        payload: serde_json::Value,
        headers: serde_json::Value,
        destination_fingerprint: &str,
    ) -> Result<Uuid> {
        Self::insert_workflow_push_enqueue(
            self,
            workflow_uuid,
            run_uuid,
            item_uuid,
            payload,
            headers,
            destination_fingerprint,
        )
        .await
    }

    async fn claim_due(&self, limit: i64, worker_id: &str) -> Result<Vec<OutboxMessage>> {
        Self::claim_due(self, limit, worker_id)
            .await
            .map(|records| {
                records
                    .into_iter()
                    .map(OutboxMessageRecord::into_message)
                    .collect()
            })
    }

    async fn next_available_at(&self) -> Result<Option<OffsetDateTime>> {
        Self::next_available_at(self).await
    }

    async fn requeue_stale_processing(&self, stale_before: OffsetDateTime) -> Result<i64> {
        Self::requeue_stale_processing(self, stale_before).await
    }

    async fn mark_delivered(&self, uuid: Uuid, locked_by: Option<&str>) -> Result<()> {
        Self::mark_delivered(self, uuid, locked_by).await
    }

    async fn mark_retry(
        &self,
        uuid: Uuid,
        last_error: &str,
        available_at: OffsetDateTime,
        locked_by: Option<&str>,
    ) -> Result<()> {
        Self::mark_retry(self, uuid, last_error, available_at, locked_by).await
    }

    async fn mark_dead_letter(
        &self,
        uuid: Uuid,
        last_error: &str,
        locked_by: Option<&str>,
    ) -> Result<()> {
        Self::mark_dead_letter(self, uuid, last_error, locked_by).await
    }
}
