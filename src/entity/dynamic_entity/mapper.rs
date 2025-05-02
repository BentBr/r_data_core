use log::{debug, error};
use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, Column, Row};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::entity::class::definition::ClassDefinition;
use crate::entity::dynamic_entity::entity::DynamicEntity;

/// Extract field data from a database row based on column types
pub fn extract_field_data(row: &PgRow) -> HashMap<String, JsonValue> {
    let mut field_data = HashMap::new();

    for column in row.columns() {
        let column_name = column.name();
        let column_type = column.type_info().to_string();

        debug!("Column: {} of type {}", column_name, column_type);

        // Handle different types based on PostgreSQL type names
        match column_type.to_lowercase().as_str() {
            // Integer types
            "int4" | "int2" => {
                if let Ok(value) = row.try_get::<Option<i32>, _>(column_name) {
                    match value {
                        Some(v) => {
                            field_data.insert(column_name.to_string(), JsonValue::Number(v.into()));
                        }
                        None => {
                            field_data.insert(column_name.to_string(), JsonValue::Null);
                        }
                    };
                } else {
                    debug!("Failed to extract int value for column: {}", column_name);
                    field_data.insert(column_name.to_string(), JsonValue::Null);
                }
            }
            "int8" => {
                if let Ok(value) = row.try_get::<Option<i64>, _>(column_name) {
                    match value {
                        Some(v) => {
                            field_data.insert(column_name.to_string(), JsonValue::Number(v.into()));
                        }
                        None => {
                            field_data.insert(column_name.to_string(), JsonValue::Null);
                        }
                    };
                } else {
                    debug!("Failed to extract bigint value for column: {}", column_name);
                    field_data.insert(column_name.to_string(), JsonValue::Null);
                }
            }
            // Float types
            "float4" | "float8" | "numeric" => {
                if let Ok(value) = row.try_get::<Option<f64>, _>(column_name) {
                    match value {
                        Some(v) => {
                            if let Some(number) = serde_json::Number::from_f64(v) {
                                field_data
                                    .insert(column_name.to_string(), JsonValue::Number(number));
                            } else {
                                field_data.insert(column_name.to_string(), JsonValue::Null);
                            }
                        }
                        None => {
                            field_data.insert(column_name.to_string(), JsonValue::Null);
                        }
                    };
                } else {
                    debug!("Failed to extract float value for column: {}", column_name);
                    field_data.insert(column_name.to_string(), JsonValue::Null);
                }
            }
            // Boolean
            "bool" => {
                if let Ok(value) = row.try_get::<Option<bool>, _>(column_name) {
                    match value {
                        Some(v) => {
                            field_data.insert(column_name.to_string(), JsonValue::Bool(v));
                        }
                        None => {
                            field_data.insert(column_name.to_string(), JsonValue::Null);
                        }
                    };
                } else {
                    debug!(
                        "Failed to extract boolean value for column: {}",
                        column_name
                    );
                    field_data.insert(column_name.to_string(), JsonValue::Null);
                }
            }
            // Text types
            "text" | "varchar" | "char" | "name" => {
                if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                    match value {
                        Some(v) => {
                            field_data.insert(column_name.to_string(), JsonValue::String(v));
                        }
                        None => {
                            field_data.insert(column_name.to_string(), JsonValue::Null);
                        }
                    };
                } else {
                    debug!("Failed to extract string value for column: {}", column_name);
                    field_data.insert(column_name.to_string(), JsonValue::Null);
                }
            }
            // UUID
            "uuid" => {
                if let Ok(value) = row.try_get::<Option<Uuid>, _>(column_name) {
                    match value {
                        Some(v) => {
                            field_data
                                .insert(column_name.to_string(), JsonValue::String(v.to_string()));
                        }
                        None => {
                            field_data.insert(column_name.to_string(), JsonValue::Null);
                        }
                    };
                } else {
                    debug!("Failed to extract UUID value for column: {}", column_name);
                    field_data.insert(column_name.to_string(), JsonValue::Null);
                }
            }
            // Timestamp types
            "timestamp" | "timestamptz" => {
                if let Ok(value) = row.try_get::<Option<OffsetDateTime>, _>(column_name) {
                    match value {
                        Some(v) => {
                            if let Ok(formatted) =
                                v.format(&time::format_description::well_known::Rfc3339)
                            {
                                field_data
                                    .insert(column_name.to_string(), JsonValue::String(formatted));
                            } else {
                                field_data.insert(column_name.to_string(), JsonValue::Null);
                            }
                        }
                        None => {
                            field_data.insert(column_name.to_string(), JsonValue::Null);
                        }
                    };
                } else {
                    debug!(
                        "Failed to extract timestamp value for column: {}",
                        column_name
                    );
                    field_data.insert(column_name.to_string(), JsonValue::Null);
                }
            }
            // Date types
            "date" => {
                if let Ok(value) = row.try_get::<Option<time::Date>, _>(column_name) {
                    match value {
                        Some(v) => {
                            field_data
                                .insert(column_name.to_string(), JsonValue::String(v.to_string()));
                        }
                        None => {
                            field_data.insert(column_name.to_string(), JsonValue::Null);
                        }
                    };
                } else {
                    debug!("Failed to extract date value for column: {}", column_name);
                    field_data.insert(column_name.to_string(), JsonValue::Null);
                }
            }
            // JSON types
            "json" | "jsonb" => {
                if let Ok(value) = row.try_get::<Option<JsonValue>, _>(column_name) {
                    match value {
                        Some(v) => {
                            field_data.insert(column_name.to_string(), v);
                        }
                        None => {
                            field_data.insert(column_name.to_string(), JsonValue::Null);
                        }
                    };
                } else {
                    debug!("Failed to extract JSON value for column: {}", column_name);
                    field_data.insert(column_name.to_string(), JsonValue::Null);
                }
            }
            // Handle NULL values for any type
            _ => {
                error!(
                    "Unsupported type extraction for column: {} of type: {}",
                    column_name, column_type
                );
            }
        }
    }

    debug!("Extracted field data: {:?}", field_data);
    field_data
}

/// Create a DynamicEntity from field data
pub fn create_entity(
    entity_type: String,
    field_data: HashMap<String, JsonValue>,
    class_def: ClassDefinition,
) -> DynamicEntity {
    DynamicEntity::from_data(entity_type, field_data, Arc::new(class_def))
}

/// Map a database row to a DynamicEntity
pub fn map_row_to_entity(
    row: &PgRow,
    entity_type: &str,
    class_def: &ClassDefinition,
) -> DynamicEntity {
    let field_data = extract_field_data(row);
    create_entity(entity_type.to_string(), field_data, class_def.clone())
}
