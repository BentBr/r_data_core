use uuid::Uuid;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::entity::field::FieldType;
use crate::entity::value::FromValue;
use super::entity::DynamicEntity;

impl DynamicEntity {
    /// Get entity UUID
    pub fn uuid(&self) -> Option<Uuid> {
        self.get::<Uuid>("uuid").ok()
    }

    /// Get entity path
    pub fn path(&self) -> Option<String> {
        self.get::<String>("path").ok()
    }

    /// Check if entity is published
    pub fn is_published(&self) -> bool {
        self.get::<bool>("published").unwrap_or(false)
    }

    /// Get entity version
    pub fn version(&self) -> i64 {
        self.get::<i64>("version").unwrap_or(1)
    }

    /// Get all custom fields
    pub fn custom_fields(&self) -> HashMap<String, Value> {
        self.data.get("custom_fields")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_else(HashMap::new)
    }

    /// Increment version when entity is updated
    pub fn increment_version(&mut self) {
        let current_version = self.version();
        self.set::<i64>("version", current_version + 1).ok();
    }

    /// Get a custom field value
    pub fn get_custom_field<T: FromValue>(&self, field: &str) -> Result<T> {
        let custom_fields = match self.data.get("custom_fields") {
            Some(Value::Object(map)) => map,
            _ => return Err(Error::FieldNotFound(format!("custom_fields.{}", field))),
        };

        let value = custom_fields.get(field).ok_or_else(|| Error::FieldNotFound(format!("custom_fields.{}", field)))?;
        T::from_value(value)
    }

    /// Validate the entity against its entity definition
    pub fn validate(&self) -> Result<()> {
        let definition = match &self.definition {
            Some(def) => def,
            None => return Ok(()), // No definition, no validation
        };

        // Check required fields
        for field_def in &definition.fields {
            if field_def.required {
                // Skip validation for relation fields - they're handled separately
                if matches!(field_def.field_type, FieldType::ManyToOne | FieldType::ManyToMany) {
                    continue;
                }

                let field_name = &field_def.name;

                // Check if the field exists in data or custom_fields
                let field_exists = self.data.contains_key(field_name) ||
                    match self.data.get("custom_fields") {
                        Some(Value::Object(map)) => map.contains_key(field_name),
                        _ => false,
                    };

                if !field_exists {
                    return Err(Error::ValidationError(format!("Required field '{}' is missing", field_name)));
                }

                // Get the value for validation
                let value = if self.data.contains_key(field_name) {
                    self.data.get(field_name).unwrap()
                } else {
                    match self.data.get("custom_fields") {
                        Some(Value::Object(map)) => map.get(field_name).unwrap(),
                        _ => return Err(Error::ValidationError(format!("Required field '{}' is missing", field_name))),
                    }
                };

                // Validate the value
                if let Err(e) = field_def.validate_value(value) {
                    return Err(Error::ValidationError(e));
                }
            }
        }

        Ok(())
    }
}
