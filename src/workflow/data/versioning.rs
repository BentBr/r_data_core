use sqlx::PgPool;
use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_persistence::WorkflowVersioningRepository;

pub async fn snapshot_workflow_pre_update(
    pool: &PgPool,
    uuid: Uuid,
    _updated_by: Option<Uuid>, // Not used - extracted from JSON
) -> Result<()> {
    let repo = WorkflowVersioningRepository::new(pool.clone());
    repo.snapshot_pre_update(uuid).await
        .map_err(|e| r_data_core_core::error::Error::Workflow(format!("Failed to snapshot workflow: {}", e)))
}
