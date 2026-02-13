use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use super::options::FieldValidation;
use super::types::FieldType;
use super::ui::UiSettings;
use crate::error::Result;

// Module re-exports
mod constraints;
mod serialization;
mod validation;
#[cfg(test)]
mod validation_tests;

/// Definition of a field in a class
#[derive(Debug, Clone, Serialize)]
pub struct FieldDefinition {
    /// Field name (must be unique within class)
    pub name: String,

    /// User-friendly display name
    pub display_name: String,

    /// Field data type
    pub field_type: FieldType,

    /// Field description for admin UI
    pub description: Option<String>,

    /// Whether the field is required
    pub required: bool,

    /// Whether the field is indexed for faster searches
    pub indexed: bool,

    /// Whether the field can be used in API filtering
    pub filterable: bool,

    /// Whether the field must have unique values (DB-level constraint)
    #[serde(default)]
    pub unique: bool,

    /// Default value for the field as JSON
    pub default_value: Option<Value>,

    /// Field validation/constraints
    #[serde(default)]
    pub validation: FieldValidation,

    /// UI settings for the field
    #[serde(default)]
    pub ui_settings: UiSettings,

    /// Extra field constraints or validation rules
    #[serde(default)]
    pub constraints: HashMap<String, Value>,
}

/// Trait to define common operations for field definitions
pub trait FieldDefinitionModule {
    /// Validate a field definition for common issues like invalid constraints
    /// # Errors
    /// Returns an error if validation fails
    fn validate(&self) -> Result<()>;

    /// Validate a value against this field definition
    /// # Errors
    /// Returns an error if validation fails
    fn validate_value(&self, value: &Value) -> Result<()>;

    /// Get the SQL type for this field
    fn get_sql_type(&self) -> String;

    /// Create a field definition with default values
    fn new_with_defaults(name: String, display_name: String, field_type: FieldType) -> Self
    where
        Self: Sized;
}

impl FieldDefinition {
    /// Create a new field definition with default values
    #[must_use]
    pub fn new(name: String, display_name: String, field_type: FieldType) -> Self {
        Self {
            name,
            display_name,
            field_type,
            description: None,
            required: false,
            indexed: false,
            filterable: false,
            unique: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        }
    }
}

// Implement FieldDefinitionModule trait
impl FieldDefinitionModule for FieldDefinition {
    fn validate(&self) -> Result<()> {
        // Delegate to the validate method
        self.validate()
    }

    fn validate_value(&self, value: &Value) -> Result<()> {
        // Delegate to the validate_value method
        self.validate_value(value)
    }

    fn get_sql_type(&self) -> String {
        // Map field type to SQL type
        match self.field_type {
            FieldType::String
            | FieldType::Text
            | FieldType::Select
            | FieldType::Wysiwyg
            | FieldType::File
            | FieldType::Image
            | FieldType::Password => "TEXT".to_string(),
            FieldType::Integer => "INTEGER".to_string(),
            FieldType::Float => "DOUBLE PRECISION".to_string(),
            FieldType::Boolean => "BOOLEAN".to_string(),
            FieldType::DateTime => "TIMESTAMP WITH TIME ZONE".to_string(),
            FieldType::Date => "DATE".to_string(),
            FieldType::Uuid | FieldType::ManyToOne => "UUID".to_string(),
            FieldType::Json | FieldType::Object | FieldType::Array => "JSONB".to_string(),
            FieldType::MultiSelect => "TEXT[]".to_string(),
            FieldType::ManyToMany => "UUID[]".to_string(),
        }
    }

    fn new_with_defaults(name: String, display_name: String, field_type: FieldType) -> Self {
        Self::new(name, display_name, field_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_definition_unique_serialization() {
        let field = FieldDefinition::new("test".to_string(), "Test".to_string(), FieldType::String);

        let json = serde_json::to_string(&field).unwrap();
        assert!(
            json.contains("\"unique\":false"),
            "JSON should contain unique field: {json}"
        );
    }

    #[test]
    fn test_field_definition_unique_true_serialization() {
        let mut field =
            FieldDefinition::new("test".to_string(), "Test".to_string(), FieldType::String);
        field.unique = true;

        let json = serde_json::to_string(&field).unwrap();
        assert!(
            json.contains("\"unique\":true"),
            "JSON should contain unique:true: {json}"
        );
    }
}
