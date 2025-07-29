use regex::Regex;
use serde_json::Value;

use crate::entity::field::definition::FieldDefinition;
use crate::entity::field::types::FieldType;
use crate::error::{Error, Result};

impl FieldDefinition {
    /// Handle a constraint validation for a field definition
    pub fn handle_constraint(&self, constraint_type: &str, constraint_value: &Value) -> Result<()> {
        match self.field_type {
            FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
                // String constraints
                match constraint_type {
                    "min_length" => {
                        validate_number_constraint(constraint_value)?;
                    }
                    "max_length" => {
                        validate_number_constraint(constraint_value)?;
                    }
                    "pattern" => {
                        validate_string_constraint(constraint_value)?;

                        // Test if pattern is a valid regex
                        let pattern = constraint_value.as_str().unwrap();
                        if let Err(e) = Regex::new(pattern) {
                            return Err(Error::Validation(format!("Invalid regex pattern: {}", e)));
                        }
                    }
                    _ => {}
                }
            }
            FieldType::Integer | FieldType::Float => {
                // Numeric constraints
                match constraint_type {
                    "min" => {
                        validate_number_constraint(constraint_value)?;
                    }
                    "max" => {
                        validate_number_constraint(constraint_value)?;
                    }
                    "precision" => {
                        validate_number_constraint(constraint_value)?;
                    }
                    "positive_only" => {
                        validate_boolean_constraint(constraint_value)?;
                    }
                    _ => {}
                }
            }
            FieldType::DateTime | FieldType::Date => {
                // Date constraints
                match constraint_type {
                    "min_date" => {
                        validate_string_constraint(constraint_value)?;
                    }
                    "max_date" => {
                        validate_string_constraint(constraint_value)?;
                    }
                    _ => {}
                }
            }
            FieldType::Select | FieldType::MultiSelect => {
                // Select constraints
                match constraint_type {
                    "options" => {
                        validate_array_constraint(constraint_value)?;
                    }
                    _ => {}
                }
            }
            FieldType::ManyToOne | FieldType::ManyToMany => {
                // Relation constraints
                match constraint_type {
                    "target_class" => {
                        validate_string_constraint(constraint_value)?;
                    }
                    _ => {}
                }
            }
            FieldType::Object | FieldType::Array | FieldType::Json => {
                // Schema constraints
                match constraint_type {
                    "schema" => {
                        validate_object_constraint(constraint_value)?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Validate a constraint value for a field
pub fn handle_constraint(
    field_type: &FieldType,
    constraint_name: &str,
    constraint_value: &Value,
) -> Result<()> {
    match field_type {
        FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
            // String constraints
            match constraint_name {
                "min_length" => {
                    validate_number_constraint(constraint_value)?;
                }
                "max_length" => {
                    validate_number_constraint(constraint_value)?;
                }
                "pattern" => {
                    validate_string_constraint(constraint_value)?;

                    // Test if pattern is a valid regex
                    let pattern = constraint_value.as_str().unwrap();
                    if let Err(e) = Regex::new(pattern) {
                        return Err(Error::Validation(format!("Invalid regex pattern: {}", e)));
                    }
                }
                _ => {}
            }
        }
        FieldType::Integer | FieldType::Float => {
            // Numeric constraints
            match constraint_name {
                "min" => {
                    validate_number_constraint(constraint_value)?;
                }
                "max" => {
                    validate_number_constraint(constraint_value)?;
                }
                "precision" => {
                    validate_number_constraint(constraint_value)?;
                }
                "positive_only" => {
                    validate_boolean_constraint(constraint_value)?;
                }
                _ => {}
            }
        }
        FieldType::DateTime | FieldType::Date => {
            // Date constraints
            match constraint_name {
                "min_date" => {
                    validate_string_constraint(constraint_value)?;
                }
                "max_date" => {
                    validate_string_constraint(constraint_value)?;
                }
                _ => {}
            }
        }
        FieldType::Select | FieldType::MultiSelect => {
            // Select constraints
            match constraint_name {
                "options" => {
                    validate_array_constraint(constraint_value)?;
                }
                _ => {}
            }
        }
        FieldType::ManyToOne | FieldType::ManyToMany => {
            // Relation constraints
            match constraint_name {
                "target_class" => {
                    validate_string_constraint(constraint_value)?;
                }
                _ => {}
            }
        }
        FieldType::Object | FieldType::Array | FieldType::Json => {
            // Schema constraints
            match constraint_name {
                "schema" => {
                    validate_object_constraint(constraint_value)?;
                }
                _ => {}
            }
        }
        _ => {}
    }

    Ok(())
}

/// Validate that a constraint value is a valid number
pub fn validate_number_constraint(constraint_value: &Value) -> Result<()> {
    if !constraint_value.is_number() {
        return Err(Error::Validation(
            "Number constraint must be a number".to_string(),
        ));
    }

    Ok(())
}

/// Validate that a constraint value is a valid string
pub fn validate_string_constraint(constraint_value: &Value) -> Result<()> {
    if !constraint_value.is_string() {
        return Err(Error::Validation(
            "String constraint must be a string".to_string(),
        ));
    }

    Ok(())
}

/// Validate that a constraint value is a valid boolean
pub fn validate_boolean_constraint(constraint_value: &Value) -> Result<()> {
    if !constraint_value.is_boolean() {
        return Err(Error::Validation(
            "Boolean constraint must be a boolean".to_string(),
        ));
    }

    Ok(())
}

/// Validate that a constraint value is a valid array
pub fn validate_array_constraint(constraint_value: &Value) -> Result<()> {
    if !constraint_value.is_array() {
        return Err(Error::Validation(
            "Array constraint must be an array".to_string(),
        ));
    }

    Ok(())
}

/// Validate that a constraint value is a valid object
pub fn validate_object_constraint(constraint_value: &Value) -> Result<()> {
    if !constraint_value.is_object() {
        return Err(Error::Validation(
            "Object constraint must be an object".to_string(),
        ));
    }

    Ok(())
}
