use sqlx::{Row};
use uuid::Uuid;

use crate::entity::dynamic_entity::utils;
use crate::error::{Error, Result};

/// Non-transactional variant using a connection pool. Use when a transaction executor is not available.
pub async fn snapshot_pre_update_pool(
    pool: &sqlx::Pool<sqlx::Postgres>,
    uuid: Uuid,
    updated_by: Option<Uuid>,
) -> Result<()> {
    // Read current entity_type and version
    let row = sqlx::query("SELECT entity_type, version FROM entities_registry WHERE uuid = $1")
        .bind(uuid)
        .fetch_optional(pool)
        .await
        .map_err(Error::Database)?;

    let (entity_type, version): (String, i32) = match row {
        Some(r) => {
            let et: String = r.try_get("entity_type").map_err(Error::Database)?;
            let v: i32 = r.try_get("version").map_err(Error::Database)?;
            (et, v)
        }
        None => return Ok(()),
    };

    // Fetch current row as JSON
    let view_name = utils::get_view_name(&entity_type);
    let current_json: Option<serde_json::Value> = sqlx::query_scalar(&format!(
        "SELECT row_to_json(t) FROM (SELECT * FROM {} WHERE uuid = $1) t",
        view_name
    ))
    .bind(uuid)
    .fetch_optional(pool)
    .await
    .map_err(Error::Database)?;

    if let Some(data_json) = current_json {
        sqlx::query(
            "INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at, created_by)
             VALUES ($1, $2, $3, $4, NOW(), $5)
             ON CONFLICT (entity_uuid, version_number) DO NOTHING",
        )
        .bind(uuid)
        .bind(&entity_type)
        .bind(version)
        .bind(data_json)
        .bind(updated_by)
        .execute(pool)
        .await
        .map_err(Error::Database)?;
    }

    Ok(())
}
