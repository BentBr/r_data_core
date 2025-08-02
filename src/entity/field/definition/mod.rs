use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use super::options::FieldValidation;
use super::types::FieldType;
use super::ui::UiSettings;
use crate::error::Result;

// Module re-exports
mod constraints;
mod schema;
mod serialization;
mod validation;

pub use constraints::*;

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
    fn validate(&self) -> Result<()>;

    /// Validate a value against this field definition
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
    pub fn new(name: String, display_name: String, field_type: FieldType) -> Self {
        Self {
            name,
            display_name,
            field_type,
            description: None,
            required: false,
            indexed: false,
            filterable: false,
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
            FieldType::String => "TEXT".to_string(),
            FieldType::Integer => "INTEGER".to_string(),
            FieldType::Float => "DOUBLE PRECISION".to_string(),
            FieldType::Boolean => "BOOLEAN".to_string(),
            FieldType::DateTime => "TIMESTAMP WITH TIME ZONE".to_string(),
            FieldType::Date => "DATE".to_string(),
            FieldType::Uuid => "UUID".to_string(),
            FieldType::Json => "JSONB".to_string(),
            FieldType::Text => "TEXT".to_string(),
            FieldType::Select => "TEXT".to_string(),
            FieldType::MultiSelect => "TEXT[]".to_string(),
            FieldType::ManyToOne => "UUID".to_string(),
            FieldType::ManyToMany => "UUID[]".to_string(),
            FieldType::Wysiwyg => "TEXT".to_string(),
            FieldType::File => "TEXT".to_string(),
            FieldType::Image => "TEXT".to_string(),
            FieldType::Object => "JSONB".to_string(),
            FieldType::Array => "JSONB".to_string(),
        }
    }

    fn new_with_defaults(name: String, display_name: String, field_type: FieldType) -> Self {
        Self::new(name, display_name, field_type)
    }
}
