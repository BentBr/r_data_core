use uuid::Uuid;

use crate::entity::version_repository::VersionRepository;
use crate::error::Result;

pub async fn snapshot_workflow_pre_update(
    pool: &sqlx::Pool<sqlx::Postgres>,
    uuid: Uuid,
    updated_by: Option<Uuid>,
) -> Result<()> {
    // Read current version
    if let Some(ver) =
        sqlx::query_scalar::<_, Option<i32>>("SELECT version FROM workflows WHERE uuid = $1")
            .bind(uuid)
            .fetch_one(pool)
            .await
            .ok()
            .flatten()
    {
        if let Ok(current_json_opt) = sqlx::query_scalar::<_, Option<serde_json::Value>>(
            "SELECT row_to_json(t) FROM (SELECT * FROM workflows WHERE uuid = $1) t",
        )
        .bind(uuid)
        .fetch_one(pool)
        .await
        {
            if let Some(current_json) = current_json_opt {
                let repo = VersionRepository::new(pool.clone());
                let _ = repo
                    .insert_snapshot(uuid, "workflow", ver, current_json, updated_by)
                    .await;
            }
        }
    }
    Ok(())
}
