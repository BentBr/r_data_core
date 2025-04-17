use super::definition::ClassDefinition;
use crate::entity::field::FieldType;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::postgres::PgPool;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub properties: HashMap<String, JsonValue>,
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }
}

impl Schema {
    pub fn new(properties: HashMap<String, JsonValue>) -> Self {
        Self { properties }
    }

    pub fn get_fields(&self) -> impl Iterator<Item = (&String, &JsonValue)> {
        self.properties.iter()
    }

    pub fn get_column_definitions(&self) -> Result<Vec<String>> {
        let mut columns = Vec::new();

        if let Some(properties) = self.properties.get("properties") {
            if let Some(props) = properties.as_object() {
                for (field_name, field_def) in props {
                    if let Some(field_type) = field_def.get("type") {
                        let sql_type = match field_type.as_str() {
                            Some("string") => {
                                if let Some(max_length) = field_def.get("maxLength") {
                                    format!("VARCHAR({})", max_length)
                                } else {
                                    "TEXT".to_string()
                                }
                            }
                            Some("number") => "NUMERIC".to_string(),
                            Some("integer") => "BIGINT".to_string(),
                            Some("boolean") => "BOOLEAN".to_string(),
                            Some("array") | Some("object") => "JSONB".to_string(),
                            Some("date") => "DATE".to_string(),
                            Some("datetime") => "TIMESTAMP WITH TIME ZONE".to_string(),
                            Some("uuid") => "UUID".to_string(),
                            _ => "TEXT".to_string(),
                        };

                        let mut column_def = format!("{} {}", field_name, sql_type);

                        if let Some(required) = field_def.get("required") {
                            if required.as_bool() == Some(true) {
                                column_def.push_str(" NOT NULL");
                            }
                        }

                        columns.push(column_def);
                    }
                }
            }
        }

        Ok(columns)
    }

    pub fn get_table_name(&self) -> String {
        if let Some(entity_type) = self.properties.get("entity_type") {
            if let Some(entity_type_str) = entity_type.as_str() {
                return format!("{}_entities", entity_type_str);
            }
        }
        "unknown_entities".to_string()
    }

    // Generate SQL DDL for this schema
    pub fn generate_schema_sql(&self) -> Result<String> {
        let table_name = self.get_table_name();
        let columns = self.get_column_definitions()?;

        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table_name);
        sql.push_str("    uuid UUID PRIMARY KEY,\n");

        for (i, column) in columns.iter().enumerate() {
            sql.push_str("    ");
            sql.push_str(column);
            if i < columns.len() - 1 || columns.is_empty() {
                sql.push_str(",\n");
            } else {
                sql.push('\n');
            }
        }

        sql.push_str(");");
        Ok(sql)
    }

    pub async fn apply_to_database(&self, pool: &PgPool) -> Result<()> {
        let sql = self.generate_schema_sql()?;
        sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(Error::Database)?;
        Ok(())
    }
}

impl From<JsonValue> for Schema {
    fn from(value: JsonValue) -> Self {
        if let JsonValue::Object(map) = value {
            Self {
                properties: map.into_iter().collect(),
            }
        } else {
            Self {
                properties: HashMap::new(),
            }
        }
    }
}

impl From<Schema> for JsonValue {
    fn from(schema: Schema) -> Self {
        JsonValue::Object(schema.properties.into_iter().collect())
    }
}
