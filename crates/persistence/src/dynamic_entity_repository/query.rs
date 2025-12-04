use log::{debug, error, warn};
use uuid::Uuid;

use crate::dynamic_entity_mapper;
use crate::dynamic_entity_utils;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

use super::DynamicEntityRepository;

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
            WHERE table_schema = 'public'
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
    let table_name = dynamic_entity_utils::get_table_name(entity_type);
    let entity_def = dynamic_entity_utils::get_entity_definition(
        &repo.pool,
        entity_type,
        repo.cache_manager.clone(),
    )
    .await?;

    // Build the query
    let query = format!(
        "SELECT e.*, r.path, r.entity_key, r.parent_uuid FROM {table_name} e
        INNER JOIN entities_registry r ON e.uuid = r.uuid
        WHERE r.entity_type = $1 AND r.parent_uuid = $2
        ORDER BY r.created_at DESC LIMIT $3 OFFSET $4"
    );

    debug!("Query by parent: {query}");

    let rows = sqlx::query(&query)
        .bind(entity_type)
        .bind(parent_uuid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&repo.pool)
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

    // Build the query
    let query = format!(
        "SELECT e.*, r.path, r.entity_key, r.parent_uuid FROM {table_name} e
        INNER JOIN entities_registry r ON e.uuid = r.uuid
        WHERE r.entity_type = $1 AND r.path = $2
        ORDER BY r.created_at DESC LIMIT $3 OFFSET $4"
    );

    debug!("Query by path: {query}");

    let rows = sqlx::query(&query)
        .bind(entity_type)
        .bind(path)
        .bind(limit)
        .bind(offset)
        .fetch_all(&repo.pool)
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

/// Get a specific entity by type and UUID
///
/// # Errors
/// Returns an error if the database query fails
pub async fn get_by_type_impl(
    repo: &DynamicEntityRepository,
    entity_type: &str,
    uuid: &Uuid,
    exclusive_fields: Option<Vec<String>>,
) -> Result<Option<DynamicEntity>> {
    debug!("Getting entity of type {entity_type} with UUID {uuid}");

    // Get the entity definition to understand entity structure
    let entity_def = dynamic_entity_utils::get_entity_definition(
        &repo.pool,
        entity_type,
        repo.cache_manager.clone(),
    )
    .await?;

    // Get the view name
    let view_name = dynamic_entity_utils::get_view_name(entity_type);

    // Build the query with field selection
    let query = exclusive_fields.map_or_else(
        || format!("SELECT * FROM {view_name} WHERE uuid = $1"),
        |fields| {
            // Always include system fields
            let mut selected_fields = vec![
                "uuid".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
                "created_by".to_string(),
                "updated_by".to_string(),
                "published".to_string(),
                "version".to_string(),
                "path".to_string(),
            ];

            // Add requested fields
            for field in fields {
                if !selected_fields.contains(&field) {
                    selected_fields.push(field.clone());
                }
            }

            format!(
                "SELECT {} FROM {view_name} WHERE uuid = $1",
                selected_fields.join(", ")
            )
        },
    );

    debug!("Query: {query}");

    let row = sqlx::query(&query)
        .bind(uuid)
        .fetch_optional(&repo.pool)
        .await
        .map_err(|e| {
            error!("Error fetching entity: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

    row.map_or_else(
        || Ok(None),
        |row| {
            // Map the row to a DynamicEntity
            let entity = dynamic_entity_mapper::map_row_to_entity(&row, entity_type, &entity_def);
            Ok(Some(entity))
        },
    )
}

/// Get all entities of a specific type with pagination
///
/// # Errors
/// Returns an error if the database query fails
pub async fn get_all_by_type_impl(
    repo: &DynamicEntityRepository,
    entity_type: &str,
    limit: i64,
    offset: i64,
    exclusive_fields: Option<Vec<String>>,
) -> Result<Vec<DynamicEntity>> {
    debug!("Getting all entities of type {entity_type}");

    // Get the entity definition to understand entity structure
    let entity_def = dynamic_entity_utils::get_entity_definition(
        &repo.pool,
        entity_type,
        repo.cache_manager.clone(),
    )
    .await?;

    // Get the view name
    let view_name = dynamic_entity_utils::get_view_name(entity_type);

    // Build the query with field selection
    let query = exclusive_fields.map_or_else(
        || format!("SELECT * FROM {view_name} ORDER BY created_at DESC LIMIT $1 OFFSET $2"),
        |fields| {
            // Always include system fields
            let mut selected_fields = vec![
                "uuid".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
                "created_by".to_string(),
                "updated_by".to_string(),
                "published".to_string(),
                "version".to_string(),
                "path".to_string(),
            ];

            // Add requested fields
            for field in fields {
                if !selected_fields.contains(&field) {
                    selected_fields.push(field.clone());
                }
            }

            format!(
                "SELECT {} FROM {view_name} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                selected_fields.join(", ")
            )
        },
    );

    debug!("Query: {query}");

    // Query all entities
    let rows = sqlx::query(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&repo.pool)
        .await
        .map_err(|e| {
            error!("Error fetching entities: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

    // Convert rows to DynamicEntity objects
    let entities = rows
        .iter()
        .map(|row| dynamic_entity_mapper::map_row_to_entity(row, entity_type, &entity_def))
        .collect();

    Ok(entities)
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
