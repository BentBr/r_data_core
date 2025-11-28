#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use std::collections::HashMap;

use super::EntityDefinitionService;

impl EntityDefinitionService {
    /// Validate entity type name
    ///
    /// # Arguments
    /// * `entity_type` - Entity type string to validate
    ///
    /// # Errors
    /// Returns an error if entity type is invalid or reserved
    pub(crate) fn validate_entity_type(&self, entity_type: &str) -> Result<()> {
        // Entity type must be alphanumeric with underscores, starting with a letter
        let valid_pattern = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();

        if !valid_pattern.is_match(entity_type) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "Entity type '{entity_type}' must start with a letter and contain only letters, numbers, and underscores"
            )));
        }

        // Check reserved words
        let reserved_words = [
            "class", "entity", "table", "column", "row", "index", "view", "schema",
        ];

        if reserved_words.contains(&entity_type.to_lowercase().as_str()) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "Entity type '{entity_type}' is a reserved word"
            )));
        }

        Ok(())
    }

    /// Validate field definitions
    ///
    /// # Arguments
    /// * `definition` - Entity definition to validate
    ///
    /// # Errors
    /// Returns an error if field validation fails
    pub(crate) fn validate_fields(&self, definition: &EntityDefinition) -> Result<()> {
        // Check for duplicate field names
        let mut field_names = HashMap::new();

        for field in &definition.fields {
            if let Some(existing) = field_names.get(&field.name.to_lowercase()) {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "Duplicate field name '{}' (previously defined at position {existing})",
                    field.name
                )));
            }

            field_names.insert(field.name.to_lowercase(), field_names.len() + 1);
        }

        // Field name must be alphanumeric with underscores, starting with a letter
        let valid_pattern = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();

        // Validate each field
        for field in &definition.fields {
            if !valid_pattern.is_match(&field.name) {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "Field name '{}' must start with a letter and contain only letters, numbers, and underscores",
                    field.name
                )));
            }

            // Additional field-specific validations can be added here
        }

        Ok(())
    }
}
