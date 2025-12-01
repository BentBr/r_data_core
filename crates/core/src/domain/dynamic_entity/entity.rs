use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::DynamicFields;
use crate::entity_definition::definition::EntityDefinition;
use crate::error::Result;
use crate::field::FieldDefinition;

// Define traits locally since value module is missing
pub trait FromValue: Sized {
    /// # Errors
    /// Returns an error if conversion fails
    fn from_value(value: &JsonValue) -> Result<Self>;
}

pub trait ToValue {
    /// # Errors
    /// Returns an error if conversion fails
    fn to_value(&self) -> Result<JsonValue>;
}

// Implement FromValue for common types
impl FromValue for String {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::String(s) => Ok(s.clone()),
            JsonValue::Number(n) => Ok(n.to_string()),
            JsonValue::Bool(b) => Ok(b.to_string()),
            JsonValue::Null => Ok(Self::new()),
            _ => Err(crate::error::Error::Conversion(format!(
                "Cannot convert {value:?} to String"
            ))),
        }
    }
}

impl FromValue for i64 {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::Number(n) => n.as_i64().ok_or_else(|| {
                crate::error::Error::Conversion(format!("Cannot convert {value:?} to i64"))
            }),
            JsonValue::String(s) => s.parse::<Self>().map_err(|_| {
                crate::error::Error::Conversion(format!("Cannot convert string '{s}' to i64"))
            }),
            _ => Err(crate::error::Error::Conversion(format!(
                "Cannot convert {value:?} to i64"
            ))),
        }
    }
}

impl FromValue for bool {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::Bool(b) => Ok(*b),
            JsonValue::Number(n) => Ok(n.as_i64().is_some_and(|x| x != 0)),
            JsonValue::String(s) => Ok(s.to_lowercase() == "true" || s == "1"),
            _ => Err(crate::error::Error::Conversion(format!(
                "Cannot convert {value:?} to bool"
            ))),
        }
    }
}

impl FromValue for JsonValue {
    fn from_value(value: &JsonValue) -> Result<Self> {
        Ok(value.clone())
    }
}

impl FromValue for Uuid {
    fn from_value(value: &JsonValue) -> Result<Self> {
        match value {
            JsonValue::String(s) => Self::parse_str(s).map_err(|_| {
                crate::error::Error::Conversion(format!("Cannot convert string '{s}' to Uuid"))
            }),
            _ => Err(crate::error::Error::Conversion(format!(
                "Cannot convert {value:?} to Uuid"
            ))),
        }
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

/// A dynamic entity that can have any fields defined by its entity definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DynamicEntity {
    /// The type of the entity
    pub entity_type: String,

    /// The field data for this entity
    pub field_data: HashMap<String, JsonValue>,

    /// The entity definition for this entity type
    #[serde(skip)]
    #[schema(skip)]
    pub definition: Arc<EntityDefinition>,
}

impl DynamicEntity {
    /// Create a new dynamic entity
    ///
    /// # Panics
    /// Panics if `OffsetDateTime::now_utc().format(&Rfc3339)` fails, which should never happen.
    #[must_use]
    pub fn new(entity_type: String, definition: Arc<EntityDefinition>) -> Self {
        let mut field_data = HashMap::new();

        // Initialize system fields
        field_data.insert(
            "uuid".to_string(),
            JsonValue::String(Uuid::now_v7().to_string()),
        );
        field_data.insert(
            "path".to_string(),
            JsonValue::String(format!("/{}", entity_type.to_lowercase())),
        );
        field_data.insert(
            "created_at".to_string(),
            JsonValue::String(OffsetDateTime::now_utc().format(&Rfc3339).unwrap()),
        );
        field_data.insert(
            "updated_at".to_string(),
            JsonValue::String(OffsetDateTime::now_utc().format(&Rfc3339).unwrap()),
        );
        field_data.insert("published".to_string(), JsonValue::Bool(false));
        field_data.insert("version".to_string(), JsonValue::Number(1.into()));

        Self {
            entity_type,
            field_data,
            definition,
        }
    }

    /// Create a dynamic entity from raw data
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: takes HashMap and Arc which are not const
    pub fn from_data(
        entity_type: String,
        field_data: HashMap<String, JsonValue>,
        definition: Arc<EntityDefinition>,
    ) -> Self {
        Self {
            entity_type,
            field_data,
            definition,
        }
    }

    /// Get a typed field value
    ///
    /// # Errors
    /// Returns `Error::Conversion` if the field value cannot be converted to the requested type.
    pub fn get<T: FromValue>(&self, field: &str) -> Result<T> {
        let value = self
            .field_data
            .get(field)
            .ok_or_else(|| crate::error::Error::FieldNotFound(field.to_string()))?;
        T::from_value(value)
    }

    /// Set a field value
    ///
    /// # Panics
    /// May panic if date formatting fails
    ///
    /// # Errors
    /// Returns an error if the field is not found or validation fails
    #[allow(clippy::needless_pass_by_value)] // value is consumed by to_value()
    pub fn set<T: ToValue>(&mut self, field: &str, value: T) -> Result<()> {
        let entity_value = value.to_value()?;

        // Check if field is defined in entity definition
        if let Some(field_def) = self.definition.get_field(field) {
            // Validate field value against definition
            if let Err(e) = field_def.validate_value(&entity_value) {
                return Err(crate::error::Error::Validation(e.to_string()));
            }
        }

        // Special handling for system fields
        match field {
            "uuid" | "created_at" => {
                // These fields are read-only after creation
                if self.field_data.contains_key(field) {
                    return Err(crate::error::Error::ReadOnlyField(field.to_string()));
                }
            }
            "updated_at" => {
                // Auto-update timestamp
                self.field_data.insert(
                    field.to_string(),
                    JsonValue::String(OffsetDateTime::now_utc().format(&Rfc3339).unwrap()),
                );
                return Ok(());
            }
            _ => {}
        }

        // Add or update the field in data
        self.field_data.insert(field.to_string(), entity_value);

        Ok(())
    }

    /// Validate the entity against its entity definition
    ///
    /// # Errors
    /// Returns `Error::Validation` if required fields are missing or field values are invalid.
    pub fn validate(&self) -> Result<()> {
        for field in &self.definition.fields {
            if field.required && !self.field_data.contains_key(&field.name) {
                return Err(crate::error::Error::Validation(format!(
                    "Required field '{}' is missing",
                    field.name
                )));
            }

            if let Some(value) = self.field_data.get(&field.name) {
                if let Err(e) = field.validate_value(value) {
                    return Err(crate::error::Error::Validation(e.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Increment the entity version number
    ///
    /// # Panics
    /// Panics if `OffsetDateTime::now_utc().format(&Rfc3339)` fails, which should never happen.
    ///
    /// # Errors
    /// Returns `Error::Conversion` if the current version cannot be read as i64.
    pub fn increment_version(&mut self) -> Result<()> {
        let current_version = self.get::<i64>("version")?;
        let new_version = current_version + 1;
        self.field_data
            .insert("version".to_string(), JsonValue::Number(new_version.into()));
        self.field_data.insert(
            "updated_at".to_string(),
            JsonValue::String(OffsetDateTime::now_utc().format(&Rfc3339).unwrap()),
        );
        Ok(())
    }

    /// Get a field definition
    #[must_use]
    pub fn get_field_definition(&self, field: &str) -> Option<&FieldDefinition> {
        self.definition.get_field(field)
    }

    /// Get all field names
    #[must_use]
    pub fn get_field_names(&self) -> Vec<String> {
        let mut fields = vec![
            "uuid".to_string(),
            "path".to_string(),
            "created_at".to_string(),
            "updated_at".to_string(),
            "published".to_string(),
            "version".to_string(),
        ];

        for field in &self.definition.fields {
            fields.push(field.name.clone());
        }

        fields
    }
}

impl DynamicFields for DynamicEntity {
    fn get_field(&self, field_name: &str) -> Option<serde_json::Value> {
        self.field_data.get(field_name).cloned()
    }

    fn set_field(
        &mut self,
        field_name: &str,
        value: serde_json::Value,
    ) -> crate::error::Result<()> {
        self.field_data.insert(field_name.to_string(), value);
        Ok(())
    }

    fn get_all_fields(&self) -> HashMap<String, serde_json::Value> {
        self.field_data.clone()
    }

    fn validate(&self, schema_properties: Option<&serde_json::Value>) -> crate::error::Result<()> {
        // For DynamicEntity, we need the full EntityDefinition, so we use the definition field
        // This is a compatibility layer - the trait signature is generic but DynamicEntity needs EntityDefinition
        let def = self.definition.as_ref();
        // Basic validation - ensure all required fields are present
        for field in &def.fields {
            if field.required && !self.field_data.contains_key(&field.name) {
                return Err(crate::error::Error::Validation(format!(
                    "Required field '{}' is missing",
                    field.name
                )));
            }

            if let Some(value) = self.field_data.get(&field.name) {
                if let Err(e) = field.validate_value(value) {
                    return Err(crate::error::Error::Validation(format!("{e}")));
                }
            }
        }

        // Fallback to generic validation if schema_properties provided (for compatibility)
        if let Some(properties) = schema_properties {
            // Fallback to generic validation if definition is not available
            if let Some(props) = properties.as_object() {
                for (field_name, field_def) in props {
                    if let Some(required) = field_def.get("required") {
                        if required.as_bool() == Some(true)
                            && !self.field_data.contains_key(field_name)
                        {
                            return Err(crate::error::Error::Validation(format!(
                                "Required field '{field_name}' is missing"
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for DynamicEntity {
    fn default() -> Self {
        Self {
            entity_type: String::new(),
            field_data: HashMap::new(),
            definition: Arc::new(EntityDefinition::default()),
        }
    }
}
