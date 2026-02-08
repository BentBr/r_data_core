use serde_json::Value as JsonValue;
use sqlx::Postgres;
use sqlx::Transaction;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::dynamic_entity_utils;
use crate::dynamic_entity_versioning;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

use super::DynamicEntityRepository;

/// Try to parse a string as an ISO 8601 / RFC 3339 timestamp
/// Returns Some(OffsetDateTime) if successful, None otherwise
fn try_parse_timestamp(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
}

/// Update an existing dynamic entity
///
/// # Errors
/// Returns an error if the database operation fails or validation fails
pub async fn update_entity(repo: &DynamicEntityRepository, entity: &DynamicEntity) -> Result<()> {
    // Validate the entity against the entity definition
    entity.validate()?;

    // Extract UUID from the entity
    let uuid =
        dynamic_entity_utils::extract_uuid_from_entity_field_data(&entity.field_data, "uuid")
            .ok_or_else(|| {
                r_data_core_core::error::Error::Validation(
                    "Entity is missing a valid UUID".to_string(),
                )
            })?;

    // Start a transaction
    let mut tx = repo.pool.begin().await?;

    // Get the current entity_type from the registry to avoid stale WHERE clauses
    let current_entity_type = sqlx::query_scalar::<_, Option<String>>(
        "SELECT entity_type FROM entities_registry WHERE uuid = $1",
    )
    .bind(uuid)
    .fetch_one(&mut *tx)
    .await?;

    // Check for internal flag to skip versioning (used by workflows with opt-out)
    let skip_versioning = entity
        .field_data
        .get("__skip_versioning")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    // Extract updated_by if present for snapshot attribution
    let updated_by =
        dynamic_entity_utils::extract_uuid_from_entity_field_data(&entity.field_data, "updated_by");
    if !skip_versioning {
        // Create snapshot BEFORE incrementing version - must be within transaction
        dynamic_entity_versioning::snapshot_pre_update(&mut tx, uuid, updated_by).await?;
    }

    // Update entities_registry table
    update_registry(&mut tx, entity, uuid).await?;

    // Update entity-specific table
    update_entity_table(&mut tx, entity, uuid, current_entity_type).await?;

    // Commit the transaction
    tx.commit().await?;

    Ok(())
}

/// Update `entities_registry` table
async fn update_registry(
    tx: &mut Transaction<'_, Postgres>,
    entity: &DynamicEntity,
    uuid: Uuid,
) -> Result<()> {
    // Collect update fields with their proper types
    let mut update_clauses = Vec::new();
    let mut param_index = 1;
    let mut path_param: Option<String> = None;
    let mut entity_key_param: Option<String> = None;
    let mut published_param: Option<bool> = None;
    let mut updated_by_param: Option<Uuid> = None;

    // Extract metadata fields for update with proper types
    if let Some(path) = entity.field_data.get("path").and_then(|v| v.as_str()) {
        update_clauses.push(format!("path = ${param_index}"));
        path_param = Some(path.to_string());
        param_index += 1;
    }

    // Include optional key update
    if let Some(entity_key) = entity.field_data.get("entity_key").and_then(|v| v.as_str()) {
        update_clauses.push(format!("entity_key = ${param_index}"));
        entity_key_param = Some(entity_key.to_string());
        param_index += 1;
    }

    if let Some(published) = entity
        .field_data
        .get("published")
        .and_then(JsonValue::as_bool)
    {
        update_clauses.push(format!("published = ${param_index}"));
        published_param = Some(published);
        param_index += 1;
    }

    let updated_by =
        dynamic_entity_utils::extract_uuid_from_entity_field_data(&entity.field_data, "updated_by");

    if let Some(item) = updated_by {
        update_clauses.push(format!("updated_by = ${param_index}"));
        updated_by_param = Some(item);
        param_index += 1;
    }

    // Always update timestamp and increment version
    let update_registry_query = if update_clauses.is_empty() {
        // Update the timestamp and version
        String::from(
            "UPDATE entities_registry SET updated_at = NOW(), version = version + 1
            WHERE uuid = $1",
        )
    } else {
        // uuid comes after the set clause params
        let uuid_pos = param_index;
        format!(
            "UPDATE entities_registry SET {}, updated_at = NOW(), version = version + 1
                WHERE uuid = ${}",
            update_clauses.join(", "),
            uuid_pos
        )
    };

    // Create a query builder
    let mut registry_query = sqlx::query(&update_registry_query);

    // Bind values for the set clauses with proper types (in parameter order)
    if let Some(path) = path_param {
        registry_query = registry_query.bind(path);
    }
    if let Some(entity_key) = entity_key_param {
        registry_query = registry_query.bind(entity_key);
    }
    if let Some(published) = published_param {
        registry_query = registry_query.bind(published);
    }
    if let Some(updated_by) = updated_by_param {
        registry_query = registry_query.bind(updated_by);
    }

    // Always bind UUID
    registry_query = registry_query.bind(uuid);

    // Execute the registry update and map unique violations
    registry_query
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(dynamic_entity_utils::map_registry_unique_violation)
}

/// Update entity-specific table
async fn update_entity_table(
    tx: &mut Transaction<'_, Postgres>,
    entity: &DynamicEntity,
    uuid: Uuid,
    current_entity_type: Option<String>,
) -> Result<()> {
    // Use current_entity_type from the registry, not entity.entity_type
    // This ensures we're updating the correct table even if entity was created as different type
    let table_name = if let Some(ref current_type) = current_entity_type {
        dynamic_entity_utils::get_table_name(current_type)
    } else {
        return Err(r_data_core_core::error::Error::Database(
            sqlx::Error::RowNotFound,
        ));
    };

    // Get column names for this table
    let valid_columns = dynamic_entity_utils::fetch_valid_columns(&mut **tx, &table_name).await?;

    // Build SET clauses for entity-specific fields with proper parameterization
    let mut set_clauses = Vec::new();
    let mut entity_params: Vec<(i32, JsonValue)> = Vec::new();
    let mut param_index = 1;

    for (key, value) in &entity.field_data {
        if dynamic_entity_utils::REGISTRY_FIELDS.contains(&key.as_str()) || key == "uuid" {
            continue; // Skip fields that are stored in entities_registry
        }
        if key == "__skip_versioning" {
            continue; // internal flag, do not persist
        }

        let key_lower = key.to_lowercase();
        if valid_columns.contains(&key_lower) {
            // Database columns are lowercase, so use lowercase for column name
            set_clauses.push(format!("{key_lower} = ${param_index}"));
            entity_params.push((param_index, value.clone()));
            param_index += 1;
        }
    }

    // Execute the entity update if we have SET clauses
    if !set_clauses.is_empty() {
        // The UUID is the last parameter
        let uuid_pos = param_index;
        let update_entity_query = format!(
            "UPDATE {} SET {} WHERE uuid = ${}",
            table_name,
            set_clauses.join(", "),
            uuid_pos
        );

        let mut entity_query = sqlx::query(&update_entity_query);

        // Bind entity-specific field values with proper types
        for (_, json_value) in &entity_params {
            if let Some(bool_val) = json_value.as_bool() {
                entity_query = entity_query.bind(bool_val);
            } else if let Some(s) = json_value.as_str() {
                // Try to parse as timestamp first (for DateTime/Date columns)
                // ISO 8601 / RFC 3339 timestamps like "2026-01-31T19:26:08.971926Z"
                if let Some(ts) = try_parse_timestamp(s) {
                    entity_query = entity_query.bind(ts);
                } else {
                    entity_query = entity_query.bind(s);
                }
            } else if let Some(n) = json_value.as_i64() {
                entity_query = entity_query.bind(n);
            } else if let Some(n) = json_value.as_f64() {
                entity_query = entity_query.bind(n);
            } else if json_value.is_null() {
                entity_query = entity_query.bind(None::<String>);
            } else {
                // For JSON objects and arrays, bind as serde_json::Value for JSONB columns
                entity_query = entity_query.bind(json_value.clone());
            }
        }

        // Always bind UUID
        entity_query = entity_query.bind(uuid);

        // Handle unique constraint violations
        entity_query
            .execute(&mut **tx)
            .await
            .map(|_| ())
            .map_err(|e| dynamic_entity_utils::map_entity_unique_violation(e, &table_name))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod try_parse_timestamp_tests {
        use super::*;

        #[test]
        fn test_parse_rfc3339_with_z() {
            let result = try_parse_timestamp("2026-01-31T19:26:08.971926Z");
            assert!(result.is_some());
            let ts = result.unwrap();
            assert_eq!(ts.year(), 2026);
            assert_eq!(ts.month() as u8, 1);
            assert_eq!(ts.day(), 31);
        }

        #[test]
        fn test_parse_rfc3339_with_offset() {
            let result = try_parse_timestamp("2026-01-31T19:26:08+00:00");
            assert!(result.is_some());
        }

        #[test]
        fn test_parse_rfc3339_no_fractional_seconds() {
            let result = try_parse_timestamp("2026-01-31T19:26:08Z");
            assert!(result.is_some());
        }

        #[test]
        fn test_parse_regular_string_returns_none() {
            let result = try_parse_timestamp("hello world");
            assert!(result.is_none());
        }

        #[test]
        fn test_parse_date_only_returns_none() {
            // Date-only strings are not valid RFC 3339
            let result = try_parse_timestamp("2026-01-31");
            assert!(result.is_none());
        }

        #[test]
        fn test_parse_uuid_returns_none() {
            let result = try_parse_timestamp("550e8400-e29b-41d4-a716-446655440000");
            assert!(result.is_none());
        }

        #[test]
        fn test_parse_empty_string_returns_none() {
            let result = try_parse_timestamp("");
            assert!(result.is_none());
        }

        #[test]
        fn test_parse_url_returns_none() {
            let result = try_parse_timestamp("https://example.com");
            assert!(result.is_none());
        }
    }
}
