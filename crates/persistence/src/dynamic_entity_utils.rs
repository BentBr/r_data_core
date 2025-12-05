use r_data_core_core::cache::CacheManager;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_core::field::FieldDefinition;
use serde_json::{self, Value as JsonValue};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Generate cache key for entity definition by entity type
fn cache_key_by_entity_type(entity_type: &str) -> String {
    format!("entity_def:by_type:{entity_type}")
}

/// Get a entity definition by entity type
///
/// # Errors
/// Returns an error if the database query fails or the entity type is not found
pub async fn get_entity_definition(
    db_pool: &PgPool,
    entity_type: &str,
    cache_manager: Option<Arc<CacheManager>>,
) -> Result<EntityDefinition> {
    // Check cache first if cache manager is provided
    if let Some(cache) = &cache_manager {
        let cache_key = cache_key_by_entity_type(entity_type);
        if let Ok(Some(cached)) = cache.get::<EntityDefinition>(&cache_key).await {
            return Ok(cached);
        }
    }
    let entity_def = sqlx::query(
        "
        SELECT entity_type, display_name, description,
               group_name, allow_children, icon, created_by,
               field_definitions
        FROM entity_definitions
        WHERE entity_type = $1
        ",
    )
    .bind(entity_type)
    .fetch_optional(db_pool)
    .await
    .map_err(r_data_core_core::error::Error::Database)?;

    // If the entity definition doesn't exist, return NotFound error
    if let Some(row) = entity_def {
        // Parse the entity definition from the row
        let fields: Vec<FieldDefinition> = serde_json::from_value(
            row.try_get("field_definitions")
                .map_err(r_data_core_core::error::Error::Database)?,
        )
        .map_err(r_data_core_core::error::Error::Serialization)?;

        let definition = EntityDefinition::from_params(
            r_data_core_core::entity_definition::definition::EntityDefinitionParams {
                entity_type: row
                    .try_get("entity_type")
                    .map_err(r_data_core_core::error::Error::Database)?,
                display_name: row
                    .try_get("display_name")
                    .map_err(r_data_core_core::error::Error::Database)?,
                description: row
                    .try_get("description")
                    .map_err(r_data_core_core::error::Error::Database)?,
                group_name: row
                    .try_get("group_name")
                    .map_err(r_data_core_core::error::Error::Database)?,
                allow_children: row
                    .try_get("allow_children")
                    .map_err(r_data_core_core::error::Error::Database)?,
                icon: row
                    .try_get("icon")
                    .map_err(r_data_core_core::error::Error::Database)?,
                fields,
                created_by: row
                    .try_get("created_by")
                    .map_err(r_data_core_core::error::Error::Database)?,
            },
        );

        // Cache the result if cache manager is provided
        if let Some(cache) = &cache_manager {
            let cache_key = cache_key_by_entity_type(entity_type);
            if let Err(e) = cache.set(&cache_key, &definition, None).await {
                log::warn!("Failed to cache entity definition: {e}");
            }
        }

        Ok(definition)
    } else {
        Err(r_data_core_core::error::Error::NotFound(format!(
            "Class definition for entity type '{entity_type}' not found"
        )))
    }
}

/// Get the view name for an entity type
#[must_use]
pub fn get_view_name(entity_type: &str) -> String {
    format!("entity_{}_view", entity_type.to_lowercase())
}

/// Get the table name for an entity type
#[must_use]
pub fn get_table_name(entity_type: &str) -> String {
    format!("entity_{}", entity_type.to_lowercase())
}

/// Build a dynamic WHERE clause from filters
#[must_use]
pub fn build_where_clause<H: std::hash::BuildHasher>(
    filters: &std::collections::HashMap<String, JsonValue, H>,
    entity_def: &EntityDefinition,
) -> (String, Vec<String>) {
    let mut where_clauses = Vec::new();
    let mut params = Vec::new();
    let mut param_idx = 1;

    // Add filters based on field types
    for (field_name, value) in filters {
        if let Some(field_def) = entity_def.get_field(field_name) {
            match field_def.field_type {
                r_data_core_core::field::types::FieldType::String
                | r_data_core_core::field::types::FieldType::Integer
                | r_data_core_core::field::types::FieldType::Float
                | r_data_core_core::field::types::FieldType::Boolean => {
                    where_clauses.push(format!("{field_name} = ${param_idx}"));
                    let param_value = match field_def.field_type {
                        r_data_core_core::field::types::FieldType::String => {
                            value.as_str().unwrap_or_default().to_string()
                        }
                        r_data_core_core::field::types::FieldType::Integer => {
                            value.as_i64().unwrap_or_default().to_string()
                        }
                        r_data_core_core::field::types::FieldType::Float => {
                            value.as_f64().unwrap_or_default().to_string()
                        }
                        r_data_core_core::field::types::FieldType::Boolean => {
                            value.as_bool().unwrap_or_default().to_string()
                        }
                        _ => unreachable!(),
                    };
                    params.push(param_value);
                }
                r_data_core_core::field::types::FieldType::DateTime
                | r_data_core_core::field::types::FieldType::Date
                | r_data_core_core::field::types::FieldType::Uuid => {
                    where_clauses.push(format!("{field_name} = ${param_idx}"));
                    params.push(value.as_str().unwrap_or_default().to_string());
                }
                _ => {
                    // For complex types (Object, Array, etc.), use JSONB comparison
                    where_clauses.push(format!("{field_name}::jsonb = ${param_idx}::jsonb"));
                    params.push(value.to_string());
                }
            }
            param_idx += 1;
        }
    }

    let clause = if where_clauses.is_empty() {
        "1=1".to_string()
    } else {
        format!("1=1 AND {}", where_clauses.join(" AND "))
    };

    (clause, params)
}

/// Extract UUID from a `JsonValue` field
/// Returns `None` if the field is not a string or if the string is not a valid UUID
#[must_use]
pub fn extract_uuid_from_json(value: &JsonValue) -> Option<Uuid> {
    match value {
        JsonValue::String(s) => Uuid::parse_str(s).ok(),
        _ => None,
    }
}

/// Extract UUID from entity field data
/// Returns None if the field is missing, not a string, or not a valid UUID
#[must_use]
pub fn extract_uuid_from_entity_field_data<H: std::hash::BuildHasher>(
    field_data: &std::collections::HashMap<String, JsonValue, H>,
    field_name: &str,
) -> Option<Uuid> {
    field_data.get(field_name).and_then(extract_uuid_from_json)
}
