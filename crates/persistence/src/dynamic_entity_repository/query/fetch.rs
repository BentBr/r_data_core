use log::{debug, error};
use uuid::Uuid;

use crate::dynamic_entity_mapper;
use crate::dynamic_entity_utils;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

use crate::dynamic_entity_repository::DynamicEntityRepository;

use super::{fetch_all_with_retry, fetch_optional_with_retry, QueryBind};

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
                    selected_fields.push(field);
                }
            }

            format!(
                "SELECT {} FROM {view_name} WHERE uuid = $1",
                selected_fields.join(", ")
            )
        },
    );

    debug!("Query: {query}");

    let row = fetch_optional_with_retry(&repo.pool, &query, vec![QueryBind::Uuid(*uuid)])
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
                    selected_fields.push(field);
                }
            }

            format!(
                "SELECT {} FROM {view_name} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                selected_fields.join(", ")
            )
        },
    );

    debug!("Query: {query}");

    // Query all entities with retry logic for schema changes
    let rows = fetch_all_with_retry(
        &repo.pool,
        &query,
        vec![QueryBind::I64(limit), QueryBind::I64(offset)],
    )
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
