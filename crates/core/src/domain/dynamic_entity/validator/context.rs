#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::Value;

use crate::error::Result;
use crate::field::FieldDefinition;

/// Encapsulates common validation parameters for a single field value.
pub struct ValidationContext<'a> {
    pub(super) field_def: &'a FieldDefinition,
    pub(super) field_name: &'a str,
    pub(super) value: &'a Value,
}

impl<'a> ValidationContext<'a> {
    #[must_use]
    pub fn new(field_def: &'a FieldDefinition, value: &'a Value) -> Self {
        Self {
            field_def,
            field_name: &field_def.name,
            value,
        }
    }

    #[must_use]
    pub const fn with_field_name(
        field_def: &'a FieldDefinition,
        value: &'a Value,
        field_name: &'a str,
    ) -> Self {
        Self {
            field_def,
            field_name,
            value,
        }
    }

    #[must_use]
    pub fn create_validation_error(&self, message: &str) -> crate::error::Error {
        crate::error::Error::Validation(format!("Field '{}' {}", self.field_name, message))
    }

    /// # Errors
    /// Returns an error if validation fails
    pub fn validate_number_range(&self, num_value: f64) -> Result<()> {
        // Range validation
        if let Some(min_value) = &self.field_def.validation.min_value {
            let min = min_value
                .as_f64()
                .ok_or_else(|| self.create_validation_error("has invalid min_value"))?;
            if num_value < min {
                return Err(self.create_validation_error(&format!("must be at least {min}")));
            }
        }

        if let Some(max_value) = &self.field_def.validation.max_value {
            let max = max_value
                .as_f64()
                .ok_or_else(|| self.create_validation_error("has invalid max_value"))?;
            if num_value > max {
                return Err(self.create_validation_error(&format!("must be no more than {max}")));
            }
        }

        // Positive only validation
        if self.field_def.validation.positive_only == Some(true) && num_value < 0.0 {
            return Err(self.create_validation_error("must be a positive number"));
        }

        Ok(())
    }

    /// # Panics
    /// May panic if value is not a string when checking for empty strings
    ///
    /// # Errors
    /// Returns an error if validation fails
    pub fn check_required(&self) -> Result<bool> {
        // Check if the field is required and the value is null or empty
        if self.field_def.required
            && (self.value.is_null() || self.value.as_str().is_some_and(str::is_empty))
        {
            return Err(self.create_validation_error("is required"));
        }

        // If the value is null and the field is not required, skip validation
        Ok(!self.value.is_null())
    }
}
