use crate::entity::class::definition::ClassDefinition;
use crate::entity::field::FieldDefinition;
use crate::error::{Error, Result};
use serde_json::{self, Value as JsonValue};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Get a class definition by entity type
pub async fn get_class_definition(db_pool: &PgPool, entity_type: &str) -> Result<ClassDefinition> {
    let class_def = sqlx::query(
        r#"
        SELECT entity_type, display_name, description, 
               group_name, allow_children, icon, created_by,
               field_definitions
        FROM class_definitions 
        WHERE entity_type = $1
        "#,
    )
    .bind(entity_type)
    .fetch_optional(db_pool)
    .await
    .map_err(Error::Database)?;

    // If the class definition doesn't exist, return NotFound error
    if let Some(row) = class_def {
        // Parse the class definition from the row
        let fields: Vec<FieldDefinition> =
            serde_json::from_value(row.try_get("field_definitions").map_err(Error::Database)?)
                .map_err(|e| Error::Serialization(e))?;

        Ok(ClassDefinition::new(
            row.try_get("entity_type").map_err(Error::Database)?,
            row.try_get("display_name").map_err(Error::Database)?,
            row.try_get("description").map_err(Error::Database)?,
            row.try_get("group_name").map_err(Error::Database)?,
            row.try_get("allow_children").map_err(Error::Database)?,
            row.try_get("icon").map_err(Error::Database)?,
            fields,
            row.try_get("created_by").map_err(Error::Database)?,
        ))
    } else {
        Err(Error::NotFound(format!(
            "Class definition for entity type '{}' not found",
            entity_type
        )))
    }
}

/// Get the view name for an entity type
pub fn get_view_name(entity_type: &str) -> String {
    format!("entity_{}_view", entity_type.to_lowercase())
}

/// Get the table name for an entity type
pub fn get_table_name(entity_type: &str) -> String {
    format!("entity_{}", entity_type.to_lowercase())
}

/// Build a dynamic WHERE clause from filters
pub fn build_where_clause(
    filters: &std::collections::HashMap<String, JsonValue>,
    class_def: &ClassDefinition,
) -> (String, Vec<String>) {
    let mut where_clauses = Vec::new();
    let mut params = Vec::new();
    let mut param_idx = 1;

    // Add filters based on field types
    for (field_name, value) in filters {
        if let Some(field_def) = class_def.get_field(field_name) {
            match field_def.field_type {
                crate::entity::field::types::FieldType::String => {
                    where_clauses.push(format!("{} = ${}", field_name, param_idx));
                    params.push(value.as_str().unwrap_or_default().to_string());
                }
                crate::entity::field::types::FieldType::Integer => {
                    where_clauses.push(format!("{} = ${}", field_name, param_idx));
                    params.push(value.as_i64().unwrap_or_default().to_string());
                }
                crate::entity::field::types::FieldType::Float => {
                    where_clauses.push(format!("{} = ${}", field_name, param_idx));
                    params.push(value.as_f64().unwrap_or_default().to_string());
                }
                crate::entity::field::types::FieldType::Boolean => {
                    where_clauses.push(format!("{} = ${}", field_name, param_idx));
                    params.push(value.as_bool().unwrap_or_default().to_string());
                }
                crate::entity::field::types::FieldType::DateTime => {
                    where_clauses.push(format!("{} = ${}", field_name, param_idx));
                    params.push(value.as_str().unwrap_or_default().to_string());
                }
                crate::entity::field::types::FieldType::Date => {
                    where_clauses.push(format!("{} = ${}", field_name, param_idx));
                    params.push(value.as_str().unwrap_or_default().to_string());
                }
                crate::entity::field::types::FieldType::Uuid => {
                    where_clauses.push(format!("{} = ${}", field_name, param_idx));
                    params.push(value.as_str().unwrap_or_default().to_string());
                }
                _ => {
                    // For complex types (Object, Array, etc.), use JSONB comparison
                    where_clauses.push(format!("{}::jsonb = ${}::jsonb", field_name, param_idx));
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

/// Extract UUID from a JsonValue field
/// Returns None if the field is not a string or if the string is not a valid UUID
pub fn extract_uuid_from_json(value: &JsonValue) -> Option<Uuid> {
    match value {
        JsonValue::String(s) => Uuid::parse_str(s).ok(),
        _ => None,
    }
}

/// Extract UUID from entity field data
/// Returns None if the field is missing, not a string, or not a valid UUID
pub fn extract_uuid_from_entity_field_data(
    field_data: &std::collections::HashMap<String, JsonValue>,
    field_name: &str,
) -> Option<Uuid> {
    field_data.get(field_name).and_then(extract_uuid_from_json)
}
