use sqlx::{Row, Transaction, Postgres};
use uuid::Uuid;

use crate::entity::dynamic_entity::utils;
use crate::entity::version_repository::VersionRepository;
use crate::error::{Error, Result};

/// Create a pre-update snapshot for a dynamic entity into entities_versions.
/// This reads the current entity_type and version from entities_registry, the current row from the
/// entity view, and inserts a snapshot at version_number with created_by set to updated_by/user.
/// This function MUST be called within a transaction before the version is incremented.
pub async fn snapshot_pre_update(
    tx: &mut Transaction<'_, Postgres>,
    uuid: Uuid,
    updated_by: Option<Uuid>,
) -> Result<()> {
    // Read current entity_type and version to snapshot as current version
    let row = sqlx::query("SELECT entity_type, version FROM entities_registry WHERE uuid = $1")
        .bind(uuid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(Error::Database)?;

    let (entity_type, version): (String, i32) = match row {
        Some(r) => {
            let et: String = r.try_get("entity_type").map_err(Error::Database)?;
            let v: i32 = r.try_get("version").map_err(Error::Database)?;
            (et, v)
        }
        None => return Ok(()), // nothing to snapshot
    };

    // Build view name and read current row as JSON
    let view_name = utils::get_view_name(&entity_type);
    let current_json: Option<serde_json::Value> = sqlx::query_scalar(&format!(
        "SELECT row_to_json(t) FROM (SELECT * FROM {} WHERE uuid = $1) t",
        view_name
    ))
    .bind(uuid)
    .fetch_optional(&mut **tx)
    .await
    .map_err(Error::Database)?;

    if let Some(data_json) = current_json {
        VersionRepository::insert_snapshot_tx(tx, uuid, &entity_type, version, data_json, updated_by).await?;
    }

    Ok(())
}
