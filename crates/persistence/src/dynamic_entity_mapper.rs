use log::{debug, error};
use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, Column, Row};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::DynamicEntity;

/// Extract an integer field value from a database row
fn extract_integer_field(row: &PgRow, column_name: &str, is_bigint: bool) -> JsonValue {
    if is_bigint {
        row.try_get::<Option<i64>, _>(column_name).map_or_else(
            |_| {
                debug!("Failed to extract bigint value for column: {column_name}");
                JsonValue::Null
            },
            |value| value.map_or(JsonValue::Null, |v| JsonValue::Number(v.into())),
        )
    } else {
        row.try_get::<Option<i32>, _>(column_name).map_or_else(
            |_| {
                debug!("Failed to extract int value for column: {column_name}");
                JsonValue::Null
            },
            |value| value.map_or(JsonValue::Null, |v| JsonValue::Number(v.into())),
        )
    }
}

/// Extract a float field value from a database row
fn extract_float_field(row: &PgRow, column_name: &str) -> JsonValue {
    row.try_get::<Option<f64>, _>(column_name).map_or_else(
        |_| {
            debug!("Failed to extract float value for column: {column_name}");
            JsonValue::Null
        },
        |value| {
            value
                .and_then(|v| serde_json::Number::from_f64(v).map(JsonValue::Number))
                .unwrap_or(JsonValue::Null)
        },
    )
}

/// Extract a boolean field value from a database row
fn extract_boolean_field(row: &PgRow, column_name: &str) -> JsonValue {
    row.try_get::<Option<bool>, _>(column_name).map_or_else(
        |_| {
            debug!("Failed to extract boolean value for column: {column_name}");
            JsonValue::Null
        },
        |value| value.map_or(JsonValue::Null, JsonValue::Bool),
    )
}

/// Extract a text field value from a database row
fn extract_text_field(row: &PgRow, column_name: &str) -> JsonValue {
    row.try_get::<Option<String>, _>(column_name).map_or_else(
        |_| {
            debug!("Failed to extract string value for column: {column_name}");
            JsonValue::Null
        },
        |value| value.map_or(JsonValue::Null, JsonValue::String),
    )
}

/// Extract a UUID field value from a database row
fn extract_uuid_field(row: &PgRow, column_name: &str) -> JsonValue {
    row.try_get::<Option<Uuid>, _>(column_name).map_or_else(
        |_| {
            debug!("Failed to extract UUID value for column: {column_name}");
            JsonValue::Null
        },
        |value| value.map_or(JsonValue::Null, |v| JsonValue::String(v.to_string())),
    )
}

/// Extract a timestamp field value from a database row
fn extract_timestamp_field(row: &PgRow, column_name: &str) -> JsonValue {
    row.try_get::<Option<OffsetDateTime>, _>(column_name)
        .map_or_else(
            |_| {
                debug!("Failed to extract timestamp value for column: {column_name}");
                JsonValue::Null
            },
            |value| {
                value
                    .and_then(|v| {
                        v.format(&time::format_description::well_known::Rfc3339)
                            .ok()
                            .map(JsonValue::String)
                    })
                    .unwrap_or(JsonValue::Null)
            },
        )
}

/// Extract a date field value from a database row
fn extract_date_field(row: &PgRow, column_name: &str) -> JsonValue {
    row.try_get::<Option<time::Date>, _>(column_name)
        .map_or_else(
            |_| {
                debug!("Failed to extract date value for column: {column_name}");
                JsonValue::Null
            },
            |value| value.map_or(JsonValue::Null, |v| JsonValue::String(v.to_string())),
        )
}

/// Extract a JSON field value from a database row
fn extract_json_field(row: &PgRow, column_name: &str) -> JsonValue {
    row.try_get::<Option<JsonValue>, _>(column_name)
        .map_or_else(
            |_| {
                debug!("Failed to extract JSON value for column: {column_name}");
                JsonValue::Null
            },
            |value| value.unwrap_or(JsonValue::Null),
        )
}

/// Extract field data from a database row based on column types
pub fn extract_field_data(row: &PgRow) -> HashMap<String, JsonValue> {
    let mut field_data = HashMap::new();
    let mut seen_columns = std::collections::HashSet::new();

    // Process columns in reverse order so later columns (like explicitly selected UUID) overwrite earlier ones
    // This handles cases where we have duplicate column names (e.g., e.uuid and r.uuid both named "uuid")
    let column_count = row.columns().len();
    for i in (0..column_count).rev() {
        let column = &row.columns()[i];
        let column_name = column.name();

        // Skip if we've already processed this column name (handles duplicates)
        // We process in reverse so the last occurrence (which appears first in SELECT) wins
        if seen_columns.contains(column_name) {
            continue;
        }
        seen_columns.insert(column_name.to_string());

        let column_type = column.type_info().to_string();

        debug!("Column: {column_name} of type {column_type}");

        let value = match column_type.to_lowercase().as_str() {
            // Integer types
            "int4" | "int2" => extract_integer_field(row, column_name, false),
            "int8" => extract_integer_field(row, column_name, true),
            // Float types
            "float4" | "float8" | "numeric" => extract_float_field(row, column_name),
            // Boolean
            "bool" => extract_boolean_field(row, column_name),
            // Text types
            "text" | "varchar" | "char" | "name" => extract_text_field(row, column_name),
            // UUID
            "uuid" => extract_uuid_field(row, column_name),
            // Timestamp types
            "timestamp" | "timestamptz" => extract_timestamp_field(row, column_name),
            // Date types
            "date" => extract_date_field(row, column_name),
            // JSON types
            "json" | "jsonb" => extract_json_field(row, column_name),
            // Handle unsupported types
            _ => {
                error!(
                    "Unsupported type extraction for column: {column_name} of type: {column_type}"
                );
                JsonValue::Null
            }
        };

        field_data.insert(column_name.to_string(), value);
    }

    debug!("Extracted field data: {field_data:?}");
    field_data
}

/// Create a `DynamicEntity` from field data
#[must_use]
pub fn create_entity<H: std::hash::BuildHasher>(
    entity_type: String,
    field_data: HashMap<String, JsonValue, H>,
    entity_def: EntityDefinition,
) -> DynamicEntity {
    let field_data: HashMap<String, JsonValue> = field_data.into_iter().collect();
    DynamicEntity::from_data(entity_type, field_data, Arc::new(entity_def))
}

/// Map a database row to a `DynamicEntity`
pub fn map_row_to_entity(
    row: &PgRow,
    entity_type: &str,
    entity_def: &EntityDefinition,
) -> DynamicEntity {
    let field_data = extract_field_data(row);

    // Map lowercase database column names back to entity definition field names (original case)
    // Database columns are lowercase, but entity definition uses original case
    let mut mapped_field_data = HashMap::new();

    // System/reserved fields that should always be kept as-is
    let system_fields = [
        "uuid",
        "created_at",
        "updated_at",
        "created_by",
        "updated_by",
        "published",
        "version",
        "path",
        "entity_key",
        "parent_uuid",
    ];

    for (db_column_name, value) in &field_data {
        // Check if this is a system field - if so, keep it as-is
        if system_fields.contains(&db_column_name.as_str()) {
            mapped_field_data.insert(db_column_name.clone(), value.clone());
        } else {
            // Find the field definition that matches this column (case-insensitive)
            if let Some(field_def) = entity_def
                .fields
                .iter()
                .find(|f| f.name.to_lowercase() == *db_column_name)
            {
                // Use the original field name from entity definition
                mapped_field_data.insert(field_def.name.clone(), value.clone());
            } else {
                // Unknown field, keep as-is
                mapped_field_data.insert(db_column_name.clone(), value.clone());
            }
        }
    }

    // Redact write-only fields (e.g. Password) so hashes are never exposed via API
    for field_def in &entity_def.fields {
        if field_def.field_type.is_write_only() {
            mapped_field_data.insert(field_def.name.clone(), JsonValue::Null);
        }
    }

    create_entity(
        entity_type.to_string(),
        mapped_field_data,
        entity_def.clone(),
    )
}
