#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::debug;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

use super::DynamicEntityService;

impl DynamicEntityService {
    /// Validate an entity against its entity definition
    ///
    /// # Arguments
    /// * `entity` - Entity to validate
    ///
    /// # Errors
    /// Returns an error if validation fails
    pub(crate) fn validate_entity(entity: &DynamicEntity) -> Result<()> {
        // Collect all validation errors instead of returning on first error
        let mut validation_errors = Vec::new();

        // Check for unknown fields - fields in the data that are not defined in the entity definition
        let unknown_fields = Self::check_unknown_fields(entity);
        if !unknown_fields.is_empty() {
            validation_errors.push(format!(
                "Unknown fields found: {}. Only fields defined in the entity definition are allowed.",
                unknown_fields.join(", ")
            ));
        }

        // For update operations, we only need to validate the fields that are being submitted
        // For create operations, check all required fields
        let is_update = Self::is_update_operation(entity);
        debug!("Validation - is update operation: {is_update}");

        if !is_update {
            // This is a create operation, so check all required fields
            Self::check_required_fields(entity, &mut validation_errors);
        }

        // Validate field values against their types and constraints (only for fields that are present)
        Self::validate_field_values(entity, &mut validation_errors);

        // If we've collected any errors, return them all as one validation error
        if !validation_errors.is_empty() {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "Validation failed with the following errors: {}",
                validation_errors.join("; ")
            )));
        }

        Ok(())
    }

    /// Check if this is an update operation based on presence of UUID
    ///
    /// # Arguments
    /// * `entity` - Entity to check
    ///
    /// # Returns
    /// `true` if this is an update operation, `false` otherwise
    fn is_update_operation(entity: &DynamicEntity) -> bool {
        entity.field_data.contains_key("uuid")
    }

    /// Check for unknown fields
    ///
    /// # Arguments
    /// * `entity` - Entity to check
    ///
    /// # Returns
    /// Vector of unknown field names
    fn check_unknown_fields(entity: &DynamicEntity) -> Vec<String> {
        let reserved_fields = [
            "uuid",
            "path",
            "parent_uuid",
            "entity_key",
            "created_at",
            "updated_at",
            "created_by",
            "updated_by",
            "published",
            "version",
        ];

        let mut unknown_fields = Vec::new();
        for field_name in entity.field_data.keys() {
            // Skip system/reserved fields
            if reserved_fields.contains(&field_name.as_str()) {
                continue;
            }

            // Check if this field exists in the entity definition (exact case match required)
            if !entity
                .definition
                .fields
                .iter()
                .any(|f| f.name == *field_name)
            {
                unknown_fields.push(field_name.clone());
            }
        }

        unknown_fields
    }

    /// Check required fields
    ///
    /// # Arguments
    /// * `entity` - Entity to check
    /// * `validation_errors` - Mutable vector to append validation errors to
    fn check_required_fields(entity: &DynamicEntity, validation_errors: &mut Vec<String>) {
        for field in &entity.definition.fields {
            if field.required && !entity.field_data.contains_key(&field.name) {
                validation_errors.push(format!("Required field '{}' is missing", field.name));
            }
        }
    }

    /// Validate field values
    ///
    /// # Arguments
    /// * `entity` - Entity to validate
    /// * `validation_errors` - Mutable vector to append validation errors to
    fn validate_field_values(entity: &DynamicEntity, validation_errors: &mut Vec<String>) {
        for field in &entity.definition.fields {
            if let Some(value) = entity.field_data.get(&field.name) {
                if let Err(e) = field.validate_value(value) {
                    validation_errors.push(format!("Field '{}' validation error: {e}", field.name));
                }
            }
        }
    }

    /// Validate an entity against its entity definition - exported for testing
    #[cfg(test)]
    pub fn validate_entity_for_test(&self, entity: &DynamicEntity) -> Result<()> {
        Self::validate_entity(entity)
    }
}
