use serde_json::Value;

use crate::error::{Error, Result};
use crate::field::definition::FieldDefinition;

impl FieldDefinition {
    /// Validate a select value
    pub(super) fn validate_select_value(&self, value: &Value) -> Result<()> {
        if !value.is_string() {
            return Err(Error::Validation(format!(
                "Field '{}' must be a string",
                self.name
            )));
        }

        // unwrap_or_default is safe: is_string() guard returned Err above
        let selected = value.as_str().unwrap_or_default();

        // Check if selected value is in options
        if let Some(crate::field::options::OptionsSource::Fixed { options }) =
            &self.validation.options_source
        {
            let valid_options: Vec<&String> = options.iter().map(|opt| &opt.value).collect();

            if !valid_options.contains(&&selected.to_string()) {
                return Err(Error::Validation(format!(
                    "Field '{}' value must be one of {:?}",
                    self.name, valid_options
                )));
            }
        }

        Ok(())
    }

    /// Validate a multiselect value
    pub(super) fn validate_multiselect_value(&self, value: &Value) -> Result<()> {
        if !value.is_array() {
            return Err(Error::Validation(format!(
                "Field '{}' must be an array",
                self.name
            )));
        }

        // map_or is safe: is_array() guard returned Err above — the None branch is unreachable
        let selected: &[Value] = value.as_array().map_or(&[], |v| v);

        // Check if all selected values are strings
        for item in selected {
            if !item.is_string() {
                return Err(Error::Validation(format!(
                    "Field '{}' must contain only string values",
                    self.name
                )));
            }
        }

        // Check if selected values are in options
        if let Some(crate::field::options::OptionsSource::Fixed { options }) =
            &self.validation.options_source
        {
            let valid_options: Vec<&String> = options.iter().map(|opt| &opt.value).collect();

            for item in selected {
                // unwrap_or_default is safe: is_string() guard returned Err for non-strings above
                let item_str = item.as_str().unwrap_or_default();
                if !valid_options.contains(&&item_str.to_string()) {
                    return Err(Error::Validation(format!(
                        "Field '{}' values must be one of {:?}",
                        self.name, valid_options
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate an array value
    pub(super) fn validate_array_value(&self, value: &Value) -> Result<()> {
        if !value.is_array() {
            return Err(Error::Validation(format!(
                "Field '{}' must be an array",
                self.name
            )));
        }

        Ok(())
    }

    /// Validate an object value
    pub(super) fn validate_object_value(&self, value: &Value) -> Result<()> {
        if !value.is_object() {
            return Err(Error::Validation(format!(
                "Field '{}' must be an object",
                self.name
            )));
        }

        Ok(())
    }
}
