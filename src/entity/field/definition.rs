use regex;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::options::FieldValidation;
use super::types::FieldType;
use super::ui::UiSettings;
use crate::error::{Error, Result};

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

// Manual implementation of Deserialize for FieldDefinition to handle constraints
impl<'de> Deserialize<'de> for FieldDefinition {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct FieldDefinitionHelper {
            pub name: String,
            pub display_name: String,
            pub field_type: FieldType,
            pub description: Option<String>,
            pub required: bool,
            pub indexed: bool,
            #[serde(default)]
            pub filterable: bool,
            pub default_value: Option<Value>,
            #[serde(default)]
            pub validation: FieldValidation,
            #[serde(default)]
            pub ui_settings: UiSettings,
            #[serde(default)]
            pub constraints: HashMap<String, Value>,
        }

        let mut helper = FieldDefinitionHelper::deserialize(deserializer)?;

        // Extract validation fields from constraints
        if let Some(pattern) = helper.constraints.get("pattern").cloned() {
            if let Some(pattern_str) = pattern.as_str() {
                helper.validation.pattern = Some(pattern_str.to_string());
            }
        }

        if let Some(min_length) = helper.constraints.get("min_length").cloned() {
            if let Some(min_len) = min_length.as_u64() {
                helper.validation.min_length = Some(min_len as usize);
            }
        }

        if let Some(max_length) = helper.constraints.get("max_length").cloned() {
            if let Some(max_len) = max_length.as_u64() {
                helper.validation.max_length = Some(max_len as usize);
            }
        }

        if let Some(min) = helper.constraints.get("min").cloned() {
            helper.validation.min_value = Some(min);
        }

        if let Some(max) = helper.constraints.get("max").cloned() {
            helper.validation.max_value = Some(max);
        }

        if let Some(positive_only) = helper.constraints.get("positive_only").cloned() {
            if let Some(positive) = positive_only.as_bool() {
                helper.validation.positive_only = Some(positive);
            }
        }

        if let Some(min_date) = helper.constraints.get("min_date").cloned() {
            if let Some(date_str) = min_date.as_str() {
                helper.validation.min_date = Some(date_str.to_string());
            }
        }

        if let Some(max_date) = helper.constraints.get("max_date").cloned() {
            if let Some(date_str) = max_date.as_str() {
                helper.validation.max_date = Some(date_str.to_string());
            }
        }

        if let Some(target_class) = helper.constraints.get("target_class").cloned() {
            if let Some(class_str) = target_class.as_str() {
                helper.validation.target_class = Some(class_str.to_string());
            }
        }

        // Handle options source for Select/MultiSelect fields
        if let Some(options) = helper.constraints.get("options").cloned() {
            if let Some(options_array) = options.as_array() {
                let mut select_options = Vec::new();
                for opt in options_array {
                    if let Some(opt_str) = opt.as_str() {
                        select_options.push(super::options::SelectOption {
                            value: opt_str.to_string(),
                            label: opt_str.to_string(),
                        });
                    }
                }

                if !select_options.is_empty() {
                    helper.validation.options_source = Some(super::options::OptionsSource::Fixed {
                        options: select_options,
                    });
                }
            }
        }

        Ok(FieldDefinition {
            name: helper.name,
            display_name: helper.display_name,
            field_type: helper.field_type,
            description: helper.description,
            required: helper.required,
            indexed: helper.indexed,
            filterable: helper.filterable,
            default_value: helper.default_value,
            validation: helper.validation,
            ui_settings: helper.ui_settings,
            constraints: helper.constraints,
        })
    }
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

    /// Convert to API schema model with properly typed constraints
    pub fn to_schema_model(
        &self,
    ) -> crate::api::admin::class_definitions::models::FieldDefinitionSchema {
        use crate::api::admin::class_definitions::models::{
            DateTimeConstraints, FieldConstraints, FieldDefinitionSchema, FieldTypeSchema,
            NumericConstraints, RelationConstraints, SchemaConstraints, SelectConstraints,
            StringConstraints, UiSettingsSchema,
        };

        // Determine the field type
        let field_type = match self.field_type {
            FieldType::String => FieldTypeSchema::String,
            FieldType::Text => FieldTypeSchema::Text,
            FieldType::Wysiwyg => FieldTypeSchema::Wysiwyg,
            FieldType::Integer => FieldTypeSchema::Integer,
            FieldType::Float => FieldTypeSchema::Float,
            FieldType::Boolean => FieldTypeSchema::Boolean,
            FieldType::DateTime => FieldTypeSchema::DateTime,
            FieldType::Date => FieldTypeSchema::Date,
            FieldType::Json => FieldTypeSchema::Object, // Map Json to Object
            FieldType::Object => FieldTypeSchema::Object,
            FieldType::Array => FieldTypeSchema::Array,
            FieldType::Uuid => FieldTypeSchema::Uuid,
            FieldType::ManyToOne => FieldTypeSchema::ManyToOne,
            FieldType::ManyToMany => FieldTypeSchema::ManyToMany,
            FieldType::Select => FieldTypeSchema::Select,
            FieldType::MultiSelect => FieldTypeSchema::MultiSelect,
            FieldType::Image => FieldTypeSchema::Image,
            FieldType::File => FieldTypeSchema::File,
        };

        // Convert constraints to typed constraints
        let constraints = match self.field_type {
            FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
                let string_constraints = StringConstraints {
                    min_length: self.validation.min_length,
                    max_length: self.validation.max_length,
                    pattern: self.validation.pattern.clone(),
                    error_message: self
                        .constraints
                        .get("error_message")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };

                if string_constraints.min_length.is_some()
                    || string_constraints.max_length.is_some()
                    || string_constraints.pattern.is_some()
                    || string_constraints.error_message.is_some()
                {
                    Some(FieldConstraints::String(string_constraints))
                } else {
                    None
                }
            }
            FieldType::Integer | FieldType::Float => {
                let numeric_constraints = NumericConstraints {
                    min: self.validation.min_value.as_ref().and_then(|v| v.as_f64()),
                    max: self.validation.max_value.as_ref().and_then(|v| v.as_f64()),
                    precision: self
                        .constraints
                        .get("precision")
                        .and_then(|v| v.as_u64())
                        .map(|p| p as u8),
                    positive_only: self.validation.positive_only,
                };

                if numeric_constraints.min.is_some()
                    || numeric_constraints.max.is_some()
                    || numeric_constraints.precision.is_some()
                    || numeric_constraints.positive_only.is_some()
                {
                    if self.field_type == FieldType::Integer {
                        Some(FieldConstraints::Integer(numeric_constraints))
                    } else {
                        Some(FieldConstraints::Float(numeric_constraints))
                    }
                } else {
                    None
                }
            }
            FieldType::DateTime | FieldType::Date => {
                let datetime_constraints = DateTimeConstraints {
                    min_date: self.validation.min_date.clone(),
                    max_date: self.validation.max_date.clone(),
                };

                if datetime_constraints.min_date.is_some()
                    || datetime_constraints.max_date.is_some()
                {
                    if self.field_type == FieldType::DateTime {
                        Some(FieldConstraints::DateTime(datetime_constraints))
                    } else {
                        Some(FieldConstraints::Date(datetime_constraints))
                    }
                } else {
                    None
                }
            }
            FieldType::Select | FieldType::MultiSelect => {
                // Extract options from options_source
                let options = match &self.validation.options_source {
                    Some(source) => match source {
                        super::options::OptionsSource::Fixed { options } => {
                            Some(options.iter().map(|opt| opt.value.clone()).collect())
                        }
                        _ => None, // Handle other option sources
                    },
                    None => self
                        .constraints
                        .get("options")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        }),
                };

                if let Some(opts) = options {
                    let select_constraints = SelectConstraints {
                        options: Some(opts),
                    };

                    if self.field_type == FieldType::Select {
                        Some(FieldConstraints::Select(select_constraints))
                    } else {
                        Some(FieldConstraints::MultiSelect(select_constraints))
                    }
                } else {
                    None
                }
            }
            FieldType::ManyToOne | FieldType::ManyToMany => {
                if let Some(target) = &self.validation.target_class {
                    let relation_constraints = RelationConstraints {
                        target_class: target.clone(),
                    };
                    Some(FieldConstraints::Relation(relation_constraints))
                } else {
                    None
                }
            }
            FieldType::Object | FieldType::Array | FieldType::Json => {
                if let Some(schema) = self.constraints.get("schema") {
                    let schema_constraints = SchemaConstraints {
                        schema: schema.clone(),
                    };
                    Some(FieldConstraints::Schema(schema_constraints))
                } else {
                    None
                }
            }
            _ => None,
        };

        // Convert UI settings
        let ui_settings = UiSettingsSchema {
            placeholder: self.ui_settings.placeholder.clone(),
            help_text: self.ui_settings.help_text.clone(),
            hide_in_lists: self.ui_settings.hide_in_lists,
            width: self.ui_settings.width,
            order: self.ui_settings.order,
            group: self.ui_settings.group.clone(),
            css_class: self.ui_settings.css_class.clone(),
            wysiwyg_toolbar: self.ui_settings.wysiwyg_toolbar.clone(),
            input_type: self.ui_settings.input_type.clone(),
        };

        FieldDefinitionSchema {
            name: self.name.clone(),
            display_name: self.display_name.clone(),
            field_type,
            description: self.description.clone(),
            required: self.required,
            indexed: self.indexed,
            filterable: self.filterable,
            default_value: self.default_value.clone(),
            constraints,
            ui_settings,
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
            FieldType::Json => {
                // For Json fields, we accept any valid JSON value
                // No additional validation needed as Value is already valid JSON
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

        // Check that field name only contains alphanumeric characters and underscores
        let name_pattern = regex::Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
        if !name_pattern.is_match(&self.name) {
            return Err(Error::Validation(
                "Field name must contain only alphanumeric characters and underscores (no spaces, hyphens, or special characters)".into(),
            ));
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

    fn handle_constraint(&self, constraint_type: &str, constraint_value: &Value) -> Result<()> {
        match self.field_type {
            FieldType::String | FieldType::Text | FieldType::Wysiwyg => match constraint_type {
                "min_length" => validate_number_constraint(constraint_value),
                "max_length" => validate_number_constraint(constraint_value),
                "pattern" => validate_string_constraint(constraint_value),
                _ => Err(Error::InvalidFieldType(format!(
                    "Invalid constraint '{}' for string field type",
                    constraint_type
                ))),
            },
            FieldType::Integer | FieldType::Float => match constraint_type {
                "min" => validate_number_constraint(constraint_value),
                "max" => validate_number_constraint(constraint_value),
                _ => Err(Error::InvalidFieldType(format!(
                    "Invalid constraint '{}' for numeric field type",
                    constraint_type
                ))),
            },
            FieldType::Array => match constraint_type {
                "min_items" => validate_number_constraint(constraint_value),
                "max_items" => validate_number_constraint(constraint_value),
                "unique_items" => validate_boolean_constraint(constraint_value),
                _ => Err(Error::InvalidFieldType(format!(
                    "Invalid constraint '{}' for array field type",
                    constraint_type
                ))),
            },
            FieldType::Select => match constraint_type {
                "options" => validate_array_constraint(constraint_value),
                _ => Err(Error::InvalidFieldType(format!(
                    "Invalid constraint '{}' for select field type",
                    constraint_type
                ))),
            },
            FieldType::Object => match constraint_type {
                "properties" => validate_object_constraint(constraint_value),
                _ => Err(Error::InvalidFieldType(format!(
                    "Invalid constraint '{}' for object field type",
                    constraint_type
                ))),
            },
            FieldType::Json => match constraint_type {
                "schema" => validate_object_constraint(constraint_value),
                _ => Err(Error::InvalidFieldType(format!(
                    "Invalid constraint '{}' for JSON field type",
                    constraint_type
                ))),
            },
            FieldType::ManyToOne | FieldType::ManyToMany => match constraint_type {
                "entity_type" => validate_string_constraint(constraint_value),
                "target_field" => validate_string_constraint(constraint_value),
                "relation_type" => validate_string_constraint(constraint_value),
                _ => Err(Error::InvalidFieldType(format!(
                    "Invalid constraint '{}' for relation field type",
                    constraint_type
                ))),
            },
            _ => Err(Error::InvalidFieldType(format!(
                "Invalid constraint '{}' for field type {:?}",
                constraint_type, self.field_type
            ))),
        }
    }
}

fn handle_constraint(
    field_type: &FieldType,
    constraint_name: &str,
    constraint_value: &Value,
) -> Result<()> {
    match field_type {
        FieldType::String | FieldType::Text | FieldType::Wysiwyg => match constraint_name {
            "min_length" | "max_length" => validate_number_constraint(constraint_value),
            "pattern" => validate_string_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for text types",
                constraint_name
            ))),
        },
        FieldType::Integer | FieldType::Float => match constraint_name {
            "min" | "max" | "step" => validate_number_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for numeric types",
                constraint_name
            ))),
        },
        FieldType::Boolean => Err(Error::InvalidFieldType(
            "Boolean type does not support constraints".to_string(),
        )),
        FieldType::DateTime | FieldType::Date => match constraint_name {
            "min" | "max" => validate_string_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for date types",
                constraint_name
            ))),
        },
        FieldType::Array => match constraint_name {
            "min_items" | "max_items" => validate_number_constraint(constraint_value),
            "allowed_types" => validate_array_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for Array type",
                constraint_name
            ))),
        },
        FieldType::Select => match constraint_name {
            "options" => validate_array_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for Select type",
                constraint_name
            ))),
        },
        FieldType::MultiSelect => match constraint_name {
            "options" => validate_array_constraint(constraint_value),
            "min_selections" | "max_selections" => validate_number_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for MultiSelect type",
                constraint_name
            ))),
        },
        FieldType::Object => match constraint_name {
            "properties" => validate_object_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for Object type",
                constraint_name
            ))),
        },
        FieldType::Json => match constraint_name {
            "schema" => validate_object_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for Json type",
                constraint_name
            ))),
        },
        FieldType::ManyToOne => match constraint_name {
            "entity_type" => validate_string_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for ManyToOne type",
                constraint_name
            ))),
        },
        FieldType::ManyToMany => match constraint_name {
            "entity_type" => validate_string_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for ManyToMany type",
                constraint_name
            ))),
        },
        FieldType::File => match constraint_name {
            "allowed_types" => validate_array_constraint(constraint_value),
            "max_size" => validate_number_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for File type",
                constraint_name
            ))),
        },
        FieldType::Uuid => Err(Error::InvalidFieldType(
            "Uuid type does not support constraints".to_string(),
        )),
        FieldType::Image => match constraint_name {
            "allowed_formats" => validate_array_constraint(constraint_value),
            "max_size" => validate_number_constraint(constraint_value),
            "max_dimensions" => validate_object_constraint(constraint_value),
            _ => Err(Error::InvalidFieldType(format!(
                "Constraint '{}' is not valid for Image type",
                constraint_name
            ))),
        },
    }
}

/// Validate that a constraint value is a valid number
fn validate_number_constraint(constraint_value: &Value) -> Result<()> {
    if constraint_value.is_number()
        || (constraint_value.is_string()
            && constraint_value.as_str().unwrap().parse::<f64>().is_ok())
    {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "Constraint value must be a number, got: {:?}",
            constraint_value
        )))
    }
}

/// Validate that a constraint value is a valid string
fn validate_string_constraint(constraint_value: &Value) -> Result<()> {
    if constraint_value.is_string() {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "Constraint value must be a string, got: {:?}",
            constraint_value
        )))
    }
}

/// Validate that a constraint value is a valid boolean
fn validate_boolean_constraint(constraint_value: &Value) -> Result<()> {
    if constraint_value.is_boolean()
        || (constraint_value.is_string()
            && ["true", "false"]
                .contains(&constraint_value.as_str().unwrap().to_lowercase().as_str()))
    {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "Constraint value must be a boolean, got: {:?}",
            constraint_value
        )))
    }
}

/// Validate that a constraint value is a valid array
fn validate_array_constraint(constraint_value: &Value) -> Result<()> {
    if constraint_value.is_array() {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "Constraint value must be an array, got: {:?}",
            constraint_value
        )))
    }
}

/// Validate that a constraint value is a valid object
fn validate_object_constraint(constraint_value: &Value) -> Result<()> {
    if constraint_value.is_object() {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "Constraint value must be an object, got: {:?}",
            constraint_value
        )))
    }
}

// Implement the trait for FieldDefinition
impl FieldDefinitionModule for FieldDefinition {
    fn validate(&self) -> Result<()> {
        // Ensure name does not contain spaces or special characters
        let valid_name_pattern = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();
        if !valid_name_pattern.is_match(&self.name) {
            return Err(Error::Validation(format!(
                "Field name '{}' must start with a letter and contain only letters, numbers, and underscores",
                self.name
            )));
        }

        // Validate constraints based on field type
        for (constraint_name, constraint_value) in &self.constraints {
            self.handle_constraint(constraint_name, constraint_value)?;
        }

        Ok(())
    }

    fn validate_value(&self, value: &Value) -> Result<()> {
        use crate::entity::dynamic_entity::validator::DynamicEntityValidator;
        DynamicEntityValidator::validate_field(self, value)
    }

    fn get_sql_type(&self) -> String {
        match self.field_type {
            FieldType::String => {
                if let Some(max_length) = self.validation.max_length {
                    if max_length <= 255 {
                        format!("VARCHAR({})", max_length)
                    } else {
                        "TEXT".to_string()
                    }
                } else {
                    "VARCHAR(255)".to_string()
                }
            }
            FieldType::Text | FieldType::Wysiwyg => "TEXT".to_string(),
            FieldType::Integer => "INTEGER".to_string(),
            FieldType::Float => "DOUBLE PRECISION".to_string(),
            FieldType::Boolean => "BOOLEAN".to_string(),
            FieldType::Date => "DATE".to_string(),
            FieldType::DateTime => "TIMESTAMP WITH TIME ZONE".to_string(),
            FieldType::Uuid => "UUID".to_string(),
            FieldType::Object | FieldType::Json | FieldType::Array => "JSONB".to_string(),
            FieldType::Select | FieldType::MultiSelect => "VARCHAR(255)".to_string(),
            FieldType::ManyToOne | FieldType::ManyToMany => "UUID".to_string(),
            FieldType::Image | FieldType::File => "VARCHAR(255)".to_string(),
        }
    }

    fn new_with_defaults(name: String, display_name: String, field_type: FieldType) -> Self {
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
