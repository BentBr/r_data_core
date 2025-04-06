use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use utoipa::ToSchema;
use serde_json::{Value as JsonValue};

use crate::error::{Error, Result};
use crate::entity::class::ClassDefinition;

// Define traits locally since value module is missing
pub trait FromValue: Sized {
    fn from_value(value: &JsonValue) -> Result<Self>;
}

pub trait ToValue {
    fn to_value(&self) -> Result<JsonValue>;
}

// Implement FromValue for common types
impl FromValue for String {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::String(s) => Ok(s.clone()),
            JsonValue::Number(n) => Ok(n.to_string()),
            JsonValue::Bool(b) => Ok(b.to_string()),
            JsonValue::Null => Ok("".to_string()),
            _ => Err(Error::Conversion(format!("Cannot convert {:?} to String", value))),
        }
    }
}

impl FromValue for i64 {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::Number(n) => n.as_i64().ok_or_else(|| Error::Conversion(format!("Cannot convert {:?} to i64", value))),
            JsonValue::String(s) => s.parse::<i64>().map_err(|_| Error::Conversion(format!("Cannot convert string '{}' to i64", s))),
            _ => Err(Error::Conversion(format!("Cannot convert {:?} to i64", value))),
        }
    }
}

impl FromValue for bool {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::Bool(b) => Ok(*b),
            JsonValue::Number(n) => Ok(n.as_i64().map(|x| x != 0).unwrap_or(false)),
            JsonValue::String(s) => Ok(s.to_lowercase() == "true" || s == "1"),
            _ => Err(Error::Conversion(format!("Cannot convert {:?} to bool", value))),
        }
    }
}

impl FromValue for JsonValue {
    fn from_value(value: &JsonValue) -> Result<Self> {
        Ok(value.clone())
    }
}

// Implement ToValue for common types
impl ToValue for String {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(JsonValue::String(self.clone()))
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(JsonValue::Number((*self).into()))
    }
}

impl ToValue for bool {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(JsonValue::Bool(*self))
    }
}

impl ToValue for JsonValue {
    fn to_value(&self) -> Result<JsonValue> {
        Ok(self.clone())
    }
}

/// A dynamic entity that can represent any entity type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DynamicEntity {
    /// Entity type name
    pub entity_type: String,
    
    /// Entity data (both standard and custom fields)
    pub data: HashMap<String, JsonValue>,
    
    /// Class definition (if available)
    #[serde(skip)]
    pub definition: Option<Arc<ClassDefinition>>,
}

impl DynamicEntity {
    /// Create a new dynamic entity
    pub fn new(entity_type: String, definition: Option<Arc<ClassDefinition>>) -> Self {
        let mut data = HashMap::new();
        
        // Initialize system fields
        data.insert("uuid".to_string(), JsonValue::String(Uuid::new_v4().to_string()));
        data.insert("path".to_string(), JsonValue::String(format!("/{}", entity_type.to_lowercase())));
        data.insert("created_at".to_string(), JsonValue::String(Utc::now().to_rfc3339()));
        data.insert("updated_at".to_string(), JsonValue::String(Utc::now().to_rfc3339()));
        data.insert("published".to_string(), JsonValue::Bool(false));
        data.insert("version".to_string(), JsonValue::Number(1.into()));
        data.insert("custom_fields".to_string(), JsonValue::Object(serde_json::Map::new()));
        
        Self {
            entity_type,
            data,
            definition,
        }
    }
    
    /// Create a dynamic entity from raw data
    pub fn from_data(
        entity_type: String, 
        data: HashMap<String, JsonValue>,
        definition: Option<Arc<ClassDefinition>>
    ) -> Self {
        Self {
            entity_type,
            data,
            definition,
        }
    }
    
    /// Get a typed field value
    pub fn get<T: FromValue>(&self, field: &str) -> Result<T> {
        let value = self.data.get(field).ok_or_else(|| Error::FieldNotFound(field.to_string()))?;
        T::from_value(value)
    }
    
    /// Set a field value
    pub fn set<T: ToValue>(&mut self, field: &str, value: T) -> Result<()> {
        let entity_value = value.to_value()?;
        
        // Validate against field definition if available
        if let Some(def) = &self.definition {
            // Check if field exists in the definition
            if let Some(field_def) = def.get_field(field) {
                // Use our validator to validate the value
                if field_def.required && matches!(entity_value, JsonValue::Null) {
                    return Err(Error::Validation(format!("Field '{}' is required", field)));
                }
                
                super::validator::DynamicEntityValidator::validate_field(field_def, &entity_value)?;
            } else if !["id", "uuid", "path", "created_at", "updated_at", "published", "version", "custom_fields"].contains(&field) {
                // If not a system field and not in definition, treat as custom field
                let custom_fields = match self.data.get("custom_fields") {
                    Some(JsonValue::Object(map)) => {
                        let mut custom_map = map.clone();
                        custom_map.insert(field.to_string(), entity_value);
                        custom_map
                    },
                    _ => {
                        let mut custom_map = serde_json::Map::new();
                        custom_map.insert(field.to_string(), entity_value);
                        custom_map
                    }
                };
                
                self.data.insert("custom_fields".to_string(), JsonValue::Object(custom_fields));
                return Ok(());
            }
        }
        
        // Special handling for system fields
        match field {
            "id" | "uuid" | "created_at" => {
                // These fields are read-only after creation
                if self.data.contains_key(field) {
                    return Err(Error::ReadOnlyField(field.to_string()));
                }
            },
            "updated_at" => {
                // Auto-update timestamp
                self.data.insert(field.to_string(), JsonValue::String(Utc::now().to_rfc3339()));
                return Ok(());
            },
            _ => {}
        }
        
        // Add or update the field in data
        self.data.insert(field.to_string(), entity_value);
        
        Ok(())
    }
    
    /// Validate the entity against its class definition
    pub fn validate(&self) -> Result<()> {
        if let Some(def) = &self.definition {
            // Check all required fields are present
            for field_def in &def.fields {
                if field_def.required {
                    // Skip system fields in validation
                    if ["id", "uuid", "path", "created_at", "updated_at", "published", "version", "custom_fields"].contains(&field_def.name.as_str()) {
                        continue;
                    }
                    
                    match self.data.get(&field_def.name) {
                        None => return Err(Error::Validation(format!("Required field '{}' is missing", field_def.name))),
                        Some(value) if matches!(value, JsonValue::Null) => {
                            return Err(Error::Validation(format!("Required field '{}' cannot be null", field_def.name)))
                        },
                        Some(value) => {
                            super::validator::DynamicEntityValidator::validate_field(field_def, value)?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Increment the entity version number
    pub fn increment_version(&mut self) -> Result<()> {
        let current_version = self.get::<i64>("version")?;
        let new_version = current_version + 1;
        self.data.insert("version".to_string(), JsonValue::Number(new_version.into()));
        self.data.insert("updated_at".to_string(), JsonValue::String(Utc::now().to_rfc3339()));
        Ok(())
    }

    pub fn default_timestamp(&mut self, field: &str) -> Result<()> {
        if !self.data.contains_key(field) {
            self.data.insert(field.to_string(), JsonValue::String(Utc::now().to_rfc3339()));
        }
        Ok(())
    }

    pub fn set_timestamp(&mut self, field: &str) -> Result<()> {
        self.data.insert(field.to_string(), JsonValue::String(Utc::now().to_rfc3339()));
        Ok(())
    }
} 