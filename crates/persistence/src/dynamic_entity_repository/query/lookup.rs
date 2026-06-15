use log::{debug, error, warn};
use uuid::Uuid;

use crate::dynamic_entity_mapper;
use crate::dynamic_entity_utils;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

use crate::dynamic_entity_repository::DynamicEntityRepository;

use super::{fetch_all_with_retry, QueryBind};

/// Count entities of a specific type
///
/// # Errors
/// Returns an error if the database query fails
pub async fn count_entities_impl(repo: &DynamicEntityRepository, entity_type: &str) -> Result<i64> {
    // Use the view for this entity type
    let view_name = dynamic_entity_utils::get_view_name(entity_type);

    // Check if view exists
    let view_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = current_schema()
            AND table_name = $1
        ) AS "exists!"
        "#,
        &view_name
    )
    .fetch_one(&repo.pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    if !view_exists {
        return Err(r_data_core_core::error::Error::NotFound(format!(
            "Entity type '{entity_type}' not found"
        )));
    }

    // Query count
    let query = format!("SELECT COUNT(*) FROM {view_name}");
    let count: i64 = sqlx::query_scalar(&query)
        .fetch_one(&repo.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

    Ok(count)
}

/// Check if an entity has children
///
/// # Errors
/// Returns an error if the database query fails
pub async fn has_children_impl(repo: &DynamicEntityRepository, parent_uuid: &Uuid) -> Result<bool> {
    let exists: Option<bool> = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM entities_registry WHERE parent_uuid = $1 LIMIT 1)",
    )
    .bind(parent_uuid)
    .fetch_one(&repo.pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    Ok(exists.unwrap_or(false))
}

/// Count children for an entity
///
/// # Errors
/// Returns an error if the database query fails
pub async fn count_children_impl(
    repo: &DynamicEntityRepository,
    parent_uuid: &Uuid,
) -> Result<i64> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entities_registry WHERE parent_uuid = $1")
            .bind(parent_uuid)
            .fetch_one(&repo.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

    Ok(count)
}

/// Find a single entity by filters
///
/// # Arguments
/// * `repo` - Repository instance
/// * `entity_type` - Type of entity to find
/// * `filters` - Map of field names to values for filtering
///
/// # Errors
/// Returns an error if the database query fails
pub async fn find_one_by_filters_impl(
    repo: &DynamicEntityRepository,
    entity_type: &str,
    filters: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<Option<DynamicEntity>> {
    use crate::dynamic_entity_repository_trait::FilterEntitiesParams;

    // Use filter_entities with limit 1 to get first match
    let params = FilterEntitiesParams::new(1, 0).with_filters(Some(filters.clone()));

    let entities =
        crate::dynamic_entity_repository::filter::filter_entities_impl(repo, entity_type, &params)
            .await?;

    Ok(entities.first().cloned())
}

/// Delete an entity by type and UUID
///
/// # Errors
/// Returns an error if the database operation fails
pub async fn delete_by_type_impl(
    repo: &DynamicEntityRepository,
    entity_type: &str,
    uuid: &Uuid,
) -> Result<()> {
    debug!("Deleting entity of type {entity_type} with UUID {uuid}");

    // Get the table name
    let table_name = dynamic_entity_utils::get_table_name(entity_type);

    // Start a transaction
    let mut tx = repo.pool.begin().await?;

    // First, delete from the entity-specific table
    let query = format!("DELETE FROM {table_name} WHERE uuid = $1");

    let result = sqlx::query(&query).bind(uuid).execute(&mut *tx).await;

    // If the entity table doesn't exist, just log a warning
    if let Err(e) = result {
        warn!("Error deleting from {table_name}: {e}");
    }

    // Then delete from entities_registry
    sqlx::query("DELETE FROM entities_registry WHERE uuid = $1 AND entity_type = $2")
        .bind(uuid)
        .bind(entity_type)
        .execute(&mut *tx)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

    // Commit the transaction
    tx.commit().await?;

    Ok(())
}

/// Get an entity by UUID without knowing the entity type
/// Searches the `entities_registry` table directly to find entity type, then queries the view
pub async fn get_by_uuid_any_type_impl(
    repo: &DynamicEntityRepository,
    uuid: &Uuid,
) -> Result<Option<DynamicEntity>> {
    // First, find the entity type from entities_registry
    let entity_type_opt: Option<String> =
        sqlx::query_scalar("SELECT entity_type FROM entities_registry WHERE uuid = $1")
            .bind(uuid)
            .fetch_optional(&repo.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

    match entity_type_opt {
        Some(entity_type) => {
            // Now query the entity-specific view to get all fields
            super::fetch::get_by_type_impl(repo, &entity_type, uuid, None).await
        }
        None => Ok(None),
    }
}

/// Query entities by `parent_uuid`
///
/// # Errors
/// Returns an error if the database query fails
pub async fn query_by_parent_impl(
    repo: &DynamicEntityRepository,
    entity_type: &str,
    parent_uuid: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<DynamicEntity>> {
    let entity_def = dynamic_entity_utils::get_entity_definition(
        &repo.pool,
        entity_type,
        repo.cache_manager.clone(),
    )
    .await?;

    // Use the view which properly handles all columns including UUID
    // The view already has UUID as r.uuid, so we don't need to worry about duplicates
    let view_name = dynamic_entity_utils::get_view_name(entity_type);

    // Build query using the view - it already has all fields properly structured
    let query = format!(
        "SELECT * FROM {view_name}
        WHERE parent_uuid = $1
        ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    );

    debug!("Query by parent: {query}");

    let rows = fetch_all_with_retry(
        &repo.pool,
        &query,
        vec![
            QueryBind::Uuid(parent_uuid),
            QueryBind::I64(limit),
            QueryBind::I64(offset),
        ],
    )
    .await
    .map_err(|e| {
        error!("Error querying entities by parent: {e:?}");
        r_data_core_core::error::Error::Database(e)
    })?;

    // Convert rows to DynamicEntity objects
    let entities = rows
        .iter()
        .map(|row| dynamic_entity_mapper::map_row_to_entity(row, entity_type, &entity_def))
        .collect();

    Ok(entities)
}

/// Query entities by exact `path`
///
/// # Errors
/// Returns an error if the database query fails
pub async fn query_by_path_impl(
    repo: &DynamicEntityRepository,
    entity_type: &str,
    path: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<DynamicEntity>> {
    let table_name = dynamic_entity_utils::get_table_name(entity_type);
    let entity_def = dynamic_entity_utils::get_entity_definition(
        &repo.pool,
        entity_type,
        repo.cache_manager.clone(),
    )
    .await?;

    // Build the query - use e.uuid explicitly to ensure it's included (e.* might not include it if there's a conflict)
    let query = format!(
        "SELECT e.*, e.uuid AS uuid, r.path, r.entity_key, r.parent_uuid FROM {table_name} e
        INNER JOIN entities_registry r ON e.uuid = r.uuid
        WHERE r.entity_type = $1 AND r.path = $2
        ORDER BY r.created_at DESC LIMIT $3 OFFSET $4"
    );

    debug!("Query by path: {query}");

    let rows = fetch_all_with_retry(
        &repo.pool,
        &query,
        vec![
            QueryBind::String(entity_type),
            QueryBind::String(path),
            QueryBind::I64(limit),
            QueryBind::I64(offset),
        ],
    )
    .await
    .map_err(|e| {
        error!("Error querying entities by path: {e:?}");
        r_data_core_core::error::Error::Database(e)
    })?;

    // Convert rows to DynamicEntity objects
    let entities = rows
        .iter()
        .map(|row| dynamic_entity_mapper::map_row_to_entity(row, entity_type, &entity_def))
        .collect();

    Ok(entities)
}
