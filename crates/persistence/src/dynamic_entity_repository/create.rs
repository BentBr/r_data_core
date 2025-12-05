use serde_json::Value as JsonValue;
use sqlx::Postgres;
use sqlx::{Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::dynamic_entity_utils;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

use super::DynamicEntityRepository;

/// Create a new dynamic entity
///
/// # Errors
/// Returns an error if the database operation fails or validation fails
pub async fn create_entity(repo: &DynamicEntityRepository, entity: &DynamicEntity) -> Result<()> {
    // Get the entity definition to validate against
    let _entity_def = dynamic_entity_utils::get_entity_definition(
        &repo.pool,
        &entity.entity_type,
        repo.cache_manager.clone(),
    )
    .await?;

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

    // Extract the path (default root) and mandatory key
    let mut path = entity
        .field_data
        .get("path")
        .and_then(|v| v.as_str().map(ToString::to_string))
        .unwrap_or_else(|| "/".to_string());

    let key = entity
        .field_data
        .get("entity_key")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            r_data_core_core::error::Error::Validation(
                "Missing required field 'entity_key'".to_string(),
            )
        })?;

    // Resolve parent_uuid, and validate/normalize path consistency
    let mut resolved_parent_uuid = dynamic_entity_utils::extract_uuid_from_entity_field_data(
        &entity.field_data,
        "parent_uuid",
    );

    // If no explicit parent given, try to infer it from the provided path by
    // checking if there exists an entity whose full path (parent.path + '/' + parent.key)
    // equals the provided path
    if resolved_parent_uuid.is_none() && path != "/" {
        if let Some((parent_uuid_found, _parent_path, _parent_key)) =
            sqlx::query_as::<_, (Uuid, String, String)>(
                "SELECT uuid, path, entity_key FROM entities_registry \
                 WHERE (CASE WHEN path = '/' THEN '/' || entity_key ELSE path || '/' || entity_key END) = $1 \
                 LIMIT 1",
            )
            .bind(&path)
            .fetch_optional(&repo.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?
        {
            resolved_parent_uuid = Some(parent_uuid_found);
        }
    }

    // Validate parent/path when we have a parent (explicit or inferred)
    if let Some(parent_uuid_val) = resolved_parent_uuid {
        // Fetch parent entity to validate path
        let parent_result: Option<(String, String)> =
            sqlx::query_as("SELECT path, entity_key FROM entities_registry WHERE uuid = $1")
                .bind(parent_uuid_val)
                .fetch_optional(&repo.pool)
                .await
                .map_err(r_data_core_core::error::Error::Database)?;

        if let Some((parent_path, parent_key)) = parent_result {
            let expected_path = if parent_path.ends_with('/') {
                format!("{parent_path}{parent_key}")
            } else {
                format!("{parent_path}/{parent_key}")
            };

            // Normalize the path to the expected parent path
            if path != expected_path {
                path = expected_path;
            }
        } else {
            return Err(r_data_core_core::error::Error::Validation(
                "Parent entity not found".to_string(),
            ));
        }
    }

    // Start a transaction
    let mut tx = repo.pool.begin().await?;

    // Insert into entities_registry
    insert_into_registry(&mut tx, entity, &uuid, &path, &key, resolved_parent_uuid).await?;

    // Insert into entity-specific table
    insert_into_entity_table(&mut tx, entity, &uuid).await?;

    // Commit the transaction
    tx.commit().await?;

    Ok(())
}

/// Insert entity metadata into `entities_registry`
async fn insert_into_registry(
    tx: &mut Transaction<'_, Postgres>,
    entity: &DynamicEntity,
    uuid: &Uuid,
    path: &str,
    key: &str,
    resolved_parent_uuid: Option<Uuid>,
) -> Result<()> {
    let registry_query = "
        INSERT INTO entities_registry
            (uuid, entity_type, path, entity_key, created_at, updated_at, created_by, updated_by, published, version, parent_uuid)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    ";

    // Extract metadata fields or use defaults
    let created_at = extract_timestamp(&entity.field_data, "created_at");
    let updated_at = extract_timestamp(&entity.field_data, "updated_at");
    let created_by =
        dynamic_entity_utils::extract_uuid_from_entity_field_data(&entity.field_data, "created_by");
    let updated_by =
        dynamic_entity_utils::extract_uuid_from_entity_field_data(&entity.field_data, "updated_by");
    let published = entity
        .field_data
        .get("published")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let version = entity
        .field_data
        .get("version")
        .and_then(JsonValue::as_i64)
        .unwrap_or(1);

    let result = sqlx::query(registry_query)
        .bind(uuid)
        .bind(&entity.entity_type)
        .bind(path)
        .bind(key)
        .bind(created_at)
        .bind(updated_at)
        .bind(created_by)
        .bind(updated_by)
        .bind(published)
        .bind(version)
        .bind(resolved_parent_uuid)
        .execute(&mut **tx)
        .await;

    // Map unique violations on (path,key) to a conflict error
    if let Err(e) = result {
        if let sqlx::Error::Database(db_err) = &e {
            // Postgres unique_violation code
            if db_err.code().as_deref() == Some("23505") {
                return Err(r_data_core_core::error::Error::ValidationFailed(
                    "An entity with the same key already exists in this path".to_string(),
                ));
            }
        }
        return Err(r_data_core_core::error::Error::Database(e));
    }

    Ok(())
}

/// Insert entity data into entity-specific table
async fn insert_into_entity_table(
    tx: &mut Transaction<'_, Postgres>,
    entity: &DynamicEntity,
    uuid: &Uuid,
) -> Result<()> {
    let table_name = dynamic_entity_utils::get_table_name(&entity.entity_type);

    // Get column names for this table
    let columns_result = sqlx::query(
        "SELECT column_name
         FROM information_schema.columns
         WHERE table_schema = current_schema() AND table_name = $1",
    )
    .bind(&table_name)
    .fetch_all(&mut **tx)
    .await?;

    // Extract column names
    let valid_columns: Vec<String> = columns_result
        .iter()
        .map(|row| {
            row.try_get::<String, _>("column_name")
                .unwrap_or_default()
                .to_lowercase()
        })
        .collect();

    // Registry fields that should not be included in the entity table
    let registry_fields = [
        "entity_type",
        "path",
        "created_at",
        "updated_at",
        "created_by",
        "updated_by",
        "published",
        "version",
    ];

    // Build query for entity-specific table
    let mut columns = vec!["uuid".to_string()];
    let mut values = vec![format!("'{}'", uuid)];

    // Add entity-specific fields
    for (key, value) in &entity.field_data {
        if registry_fields.contains(&key.as_str()) || key == "uuid" {
            continue; // Skip fields that are stored in entities_registry
        }

        let key_lower = key.to_lowercase();
        if valid_columns.contains(&key_lower) {
            // Database columns are lowercase, so use lowercase for column name
            columns.push(key_lower);

            // Format the value appropriately based on its type
            let value_str = format_value_for_sql(value);
            values.push(value_str);
        }
    }

    // Create the INSERT statement for the entity table
    if columns.len() > 1 {
        // If we have more than just the UUID
        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            values.join(", ")
        );

        sqlx::query(&query).execute(&mut **tx).await?;
    } else {
        // If we only have the UUID, just insert that
        sqlx::query(&format!("INSERT INTO {table_name} (uuid) VALUES ($1)"))
            .bind(uuid)
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

/// Extract timestamp from field data
fn extract_timestamp(
    field_data: &std::collections::HashMap<String, JsonValue>,
    key: &str,
) -> OffsetDateTime {
    field_data
        .get(key)
        .and_then(|v| v.as_str())
        .map_or_else(OffsetDateTime::now_utc, |s| {
            OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| OffsetDateTime::now_utc())
        })
}

/// Format JSON value for SQL insertion
fn format_value_for_sql(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => format!("'{}'", s.replace('\'', "''")),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        JsonValue::Null => "NULL".to_string(),
        _ => format!("'{}'", value.to_string().replace('\'', "''")), // For complex types
    }
}
