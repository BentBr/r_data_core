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

/// Registry fields that should not be included in entity-specific tables
pub const REGISTRY_FIELDS: &[&str] = &[
    "entity_type",
    "path",
    "created_at",
    "updated_at",
    "created_by",
    "updated_by",
    "published",
    "version",
];

/// Fetch valid column names for a given table from `information_schema`
///
/// # Errors
/// Returns an error if the database query fails
pub async fn fetch_valid_columns<'e, E>(executor: E, table_name: &str) -> Result<Vec<String>>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let columns_result = sqlx::query(
        "SELECT column_name
         FROM information_schema.columns
         WHERE table_schema = current_schema() AND table_name = $1",
    )
    .bind(table_name)
    .fetch_all(executor)
    .await?;

    Ok(columns_result
        .iter()
        .map(|row| {
            row.try_get::<String, _>("column_name")
                .unwrap_or_default()
                .to_lowercase()
        })
        .collect())
}

/// Extract field name from a unique constraint name
/// Constraint format: `idx_{table}_{field}_unique`
#[must_use]
pub fn extract_field_from_unique_constraint(constraint: Option<&str>, table_name: &str) -> String {
    if let Some(constraint_name) = constraint {
        let prefix = format!("idx_{table_name}_");
        let suffix = "_unique";
        if constraint_name.starts_with(&prefix) && constraint_name.ends_with(suffix) {
            return constraint_name[prefix.len()..constraint_name.len() - suffix.len()].to_string();
        }
    }
    "unknown".to_string()
}

/// Map a sqlx unique constraint violation on the `entities_registry` (path + key) to a validation
/// error. Non-unique-violation errors are mapped to `Error::Database`.
#[must_use]
pub fn map_registry_unique_violation(err: sqlx::Error) -> r_data_core_core::error::Error {
    if let sqlx::Error::Database(ref db_err) = err {
        if db_err.code().as_deref() == Some("23505") {
            return r_data_core_core::error::Error::ValidationFailed(
                "An entity with the same key already exists in this path".to_string(),
            );
        }
    }
    r_data_core_core::error::Error::Database(err)
}

/// Map a sqlx unique constraint violation on an entity-specific table to a validation error,
/// extracting the field name from the constraint.
/// Non-unique-violation errors are mapped to `Error::Database`.
#[must_use]
pub fn map_entity_unique_violation(
    err: sqlx::Error,
    table_name: &str,
) -> r_data_core_core::error::Error {
    if let sqlx::Error::Database(ref db_err) = err {
        if db_err.code().as_deref() == Some("23505") {
            let field_name = extract_field_from_unique_constraint(db_err.constraint(), table_name);
            return r_data_core_core::error::Error::ValidationFailed(format!(
                "Field '{field_name}' must be unique. A record with this value already exists."
            ));
        }
    }
    r_data_core_core::error::Error::Database(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod extract_field_from_unique_constraint_tests {
        use super::*;

        #[test]
        fn test_extract_field_from_valid_constraint() {
            let result = extract_field_from_unique_constraint(
                Some("idx_entity_customer_email_unique"),
                "entity_customer",
            );
            assert_eq!(result, "email");
        }

        #[test]
        fn test_extract_field_from_constraint_with_underscores() {
            let result = extract_field_from_unique_constraint(
                Some("idx_entity_test_my_field_unique"),
                "entity_test",
            );
            assert_eq!(result, "my_field");
        }

        #[test]
        fn test_extract_field_from_none_constraint() {
            let result = extract_field_from_unique_constraint(None, "entity_test");
            assert_eq!(result, "unknown");
        }

        #[test]
        fn test_extract_field_from_mismatched_prefix() {
            let result = extract_field_from_unique_constraint(
                Some("idx_other_table_field_unique"),
                "entity_test",
            );
            assert_eq!(result, "unknown");
        }

        #[test]
        fn test_extract_field_from_missing_suffix() {
            let result =
                extract_field_from_unique_constraint(Some("idx_entity_test_field"), "entity_test");
            assert_eq!(result, "unknown");
        }
    }
}
