use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
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
                            Some("array" | "object") => "JSONB".to_string(),
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
