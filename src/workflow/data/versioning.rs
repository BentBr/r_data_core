use sqlx::PgPool;
use uuid::Uuid;

use crate::error::Result;
use crate::workflow::data::versioning_repository::WorkflowVersioningRepository;

pub async fn snapshot_workflow_pre_update(
    pool: &PgPool,
    uuid: Uuid,
    _updated_by: Option<Uuid>, // Not used - extracted from JSON
) -> Result<()> {
    let repo = WorkflowVersioningRepository::new(pool.clone());
    repo.snapshot_pre_update(uuid).await
}
