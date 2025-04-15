use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::options::FieldValidation;
use super::types::FieldType;
use super::ui::UiSettings;
use crate::error::{Error, Result};

/// Definition of a field in a class
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub constraints: HashMap<String, Value>,
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

    /// Validate a field value against this definition
    pub fn validate_value(&self, value: &Value) -> Result<()> {
        // Check if required
        if self.required && value.is_null() {
            return Err(Error::Validation(format!(
                "Field '{}' is required",
                self.name
            )));
        }

        // Skip validation for null values if not required
        if value.is_null() {
            return Ok(());
        }

        // Type-specific validation
        match self.field_type {
            FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
                if !value.is_string() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be a string",
                        self.name
                    )));
                }
                if let Some(max_length) = self.validation.max_length {
                    if value.as_str().unwrap().len() > max_length {
                        return Err(Error::Validation(format!(
                            "Field '{}' exceeds maximum length of {}",
                            self.name, max_length
                        )));
                    }
                }
            }
            FieldType::Integer => {
                if !value.is_i64() && !value.is_u64() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be an integer",
                        self.name
                    )));
                }
            }
            FieldType::Float => {
                if !value.is_f64() && !value.is_i64() && !value.is_u64() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be a number",
                        self.name
                    )));
                }
            }
            FieldType::Boolean => {
                if !value.is_boolean() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be a boolean",
                        self.name
                    )));
                }
            }
            FieldType::DateTime | FieldType::Date => {
                if !value.is_string() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be a date string",
                        self.name
                    )));
                }
            }
            FieldType::Uuid => {
                if !value.is_string() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be a UUID string",
                        self.name
                    )));
                }
            }
            FieldType::Select | FieldType::MultiSelect => {
                if !value.is_string() && !value.is_array() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be a string or array",
                        self.name
                    )));
                }
            }
            FieldType::Object => {
                if !value.is_object() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be an object",
                        self.name
                    )));
                }
            }
            FieldType::Array => {
                if !value.is_array() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be an array",
                        self.name
                    )));
                }
            }
            FieldType::ManyToOne | FieldType::ManyToMany => {
                if !value.is_string() && !value.is_array() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be a UUID string or array of UUIDs",
                        self.name
                    )));
                }
            }
            FieldType::Image | FieldType::File => {
                if !value.is_string() {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be a file path string",
                        self.name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get the SQL type for this field
    pub fn get_sql_type(&self) -> String {
        super::types::get_sql_type_for_field(
            &self.field_type,
            self.validation.max_length,
            None, // TODO: Add enum name support
        )
    }

    /// Validate the field definition itself
    pub fn validate(&self) -> Result<()> {
        // Name validation
        if self.name.is_empty() {
            return Err(Error::Validation("Field name cannot be empty".into()));
        }

        // Display name validation
        if self.display_name.is_empty() {
            return Err(Error::Validation(
                "Field display name cannot be empty".into(),
            ));
        }

        // Validate constraints based on field type
        match self.field_type {
            FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
                if let (Some(min), Some(max)) =
                    (self.validation.min_length, self.validation.max_length)
                {
                    if min > max {
                        return Err(Error::Validation(format!(
                            "Field '{}' min_length cannot be greater than max_length",
                            self.name
                        )));
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
