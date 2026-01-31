use log::{debug, error, warn};
use sqlx::PgPool;
use uuid::Uuid;

use crate::dynamic_entity_mapper;
use crate::dynamic_entity_utils;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

use super::DynamicEntityRepository;

/// Check if an error is the "cached plan must not change result type" error
/// This occurs when cached plan types change (aka an entity definition changes) and not every connection has gotten the updated cache yet
fn is_cached_plan_error(err: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = err {
        // PostgreSQL error code 0A000 = feature_not_supported
        // This specific message occurs when cached plan types change
        db_err.code().is_some_and(|c| c == "0A000")
            && db_err
                .message()
                .contains("cached plan must not change result type")
    } else {
        false
    }
}

/// Execute a query with retry logic for cached plan errors.
/// On the first "cached plan must not change result type" error,
/// we run DISCARD PLANS to clear the statement cache and retry once.
async fn fetch_all_with_retry<'q>(
    pool: &PgPool,
    query: &'q str,
    binds: Vec<QueryBind<'q>>,
) -> std::result::Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> {
    let result = execute_query(pool, query, &binds).await;

    match result {
        Err(ref e) if is_cached_plan_error(e) => {
            warn!("Cached plan error detected, discarding plans and retrying query");

            // Clear cached plans on this connection
            sqlx::query("DISCARD PLANS").execute(pool).await?;

            // Retry the query
            execute_query(pool, query, &binds).await
        }
        other => other,
    }
}

/// Execute a query with retry logic for cached plan errors (single row).
async fn fetch_optional_with_retry<'q>(
    pool: &PgPool,
    query: &'q str,
    binds: Vec<QueryBind<'q>>,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    let result = execute_query_optional(pool, query, &binds).await;

    match result {
        Err(ref e) if is_cached_plan_error(e) => {
            warn!("Cached plan error detected, discarding plans and retrying query");

            // Clear cached plans on this connection
            sqlx::query("DISCARD PLANS").execute(pool).await?;

            // Retry the query
            execute_query_optional(pool, query, &binds).await
        }
        other => other,
    }
}

/// Query bind value types we support
#[derive(Clone)]
enum QueryBind<'a> {
    Uuid(Uuid),
    I64(i64),
    String(&'a str),
}

/// Execute a query with binds
async fn execute_query<'q>(
    pool: &PgPool,
    query: &'q str,
    binds: &[QueryBind<'q>],
) -> std::result::Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> {
    let mut q = sqlx::query(query);
    for bind in binds {
        q = match bind {
            QueryBind::Uuid(v) => q.bind(*v),
            QueryBind::I64(v) => q.bind(*v),
            QueryBind::String(v) => q.bind(*v),
        };
    }
    q.fetch_all(pool).await
}

/// Execute a query with binds (optional result)
async fn execute_query_optional<'q>(
    pool: &PgPool,
    query: &'q str,
    binds: &[QueryBind<'q>],
) -> std::result::Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    let mut q = sqlx::query(query);
    for bind in binds {
        q = match bind {
            QueryBind::Uuid(v) => q.bind(*v),
            QueryBind::I64(v) => q.bind(*v),
            QueryBind::String(v) => q.bind(*v),
        };
    }
    q.fetch_optional(pool).await
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_cached_plan_error_returns_false_for_non_database_errors() {
        // Protocol error - not a database error
        let err = sqlx::Error::Protocol("some protocol error".to_string());
        assert!(!is_cached_plan_error(&err));

        // Row not found error
        let err = sqlx::Error::RowNotFound;
        assert!(!is_cached_plan_error(&err));

        // Configuration error
        let err = sqlx::Error::Configuration("bad config".into());
        assert!(!is_cached_plan_error(&err));
    }

    #[test]
    fn query_bind_enum_is_clone() {
        // Verify QueryBind implements Clone (required for retry logic)
        let uuid_bind = QueryBind::Uuid(uuid::Uuid::nil());
        let _cloned = uuid_bind.clone();

        let i64_bind = QueryBind::I64(42);
        let _cloned = i64_bind.clone();

        let str_bind = QueryBind::String("test");
        let _cloned = str_bind.clone();
    }
}
