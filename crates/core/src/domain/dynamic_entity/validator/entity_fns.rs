#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::Value;

use crate::entity_definition::definition::EntityDefinition;
use crate::error::Result;

use super::context::ValidationContext;
use super::entity_validator::DynamicEntityValidator;

/// Represents a field-specific validation error
#[derive(Debug, Clone)]
pub struct FieldViolation {
    pub field: String,
    pub message: String,
}

/// # Errors
/// Returns an error if validation fails
pub fn validate_entity(entity: &Value, entity_def: &EntityDefinition) -> Result<()> {
    let violations = validate_entity_with_violations(entity, entity_def)?;
    if !violations.is_empty() {
        return Err(crate::error::Error::Validation(format!(
            "Validation failed with the following errors: {}",
            violations
                .iter()
                .map(|v| format!("Field '{}': {}", v.field, v.message))
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }

    Ok(())
}

/// Validate entity and return structured violations
///
/// # Errors
/// Returns an error if validation fails
pub fn validate_entity_with_violations(
    entity: &Value,
    entity_def: &EntityDefinition,
) -> Result<Vec<FieldViolation>> {
    let mut violations = Vec::new();
    let entity_type = entity
        .get("entity_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            crate::error::Error::Validation("Entity must have an entity_type field".to_string())
        })?;

    if entity_type != entity_def.entity_type {
        return Err(crate::error::Error::Validation(format!(
            "Entity type '{}' does not match entity definition type '{}'",
            entity_type, entity_def.entity_type
        )));
    }

    let field_data = entity
        .get("field_data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            crate::error::Error::Validation("Entity must have a field_data object".to_string())
        })?;

    // Check required fields
    for field_def in &entity_def.fields {
        if field_def.required && !field_data.contains_key(&field_def.name) {
            violations.push(FieldViolation {
                field: field_def.name.clone(),
                message: "This field is required".to_string(),
            });
        }
    }

    // Validate fields that are present
    for (field_name, value) in field_data {
        if let Some(field_def) = entity_def.get_field(field_name) {
            let _ = ValidationContext::with_field_name(field_def, value, field_name);
            if let Err(e) = DynamicEntityValidator::validate_field(field_def, value) {
                // Extract just the inner message from the Error::Validation variant
                // and strip the "Field 'x' " prefix if present for cleaner violation messages
                let message = match e {
                    crate::error::Error::Validation(msg) => {
                        // Strip "Field 'field_name' " prefix if present
                        let prefix = format!("Field '{field_name}' ");
                        msg.strip_prefix(&prefix).unwrap_or(&msg).to_string()
                    }
                    other => other.to_string(),
                };
                violations.push(FieldViolation {
                    field: field_name.clone(),
                    message,
                });
            }
        } else {
            // Skip system fields
            let system_fields = [
                "uuid",
                "entity_key",
                "path",
                "created_at",
                "updated_at",
                "created_by",
                "updated_by",
                "published",
                "version",
                "parent_uuid", // Parent entity reference
            ];
            if !system_fields.contains(&field_name.as_str()) {
                violations.push(FieldViolation {
                    field: field_name.clone(),
                    message: "This field is not defined in the entity definition".to_string(),
                });
            }
        }
    }

    Ok(violations)
}

/// Validate that `parent_uuid` and path are consistent
/// Returns Ok(()) if valid, or adds violations if invalid
/// This function checks the relationship between `parent_uuid` and path
///
/// # Errors
/// Returns an error if validation processing fails (should not happen in normal operation).
pub fn validate_parent_path_consistency(
    parent_uuid: Option<String>,
    path: Option<&String>,
    expected_path: Option<&String>,
) -> Result<Vec<FieldViolation>> {
    let mut violations = Vec::new();

    // If parent_uuid is set, we need to validate the path
    if let Some(parent_uuid_str) = parent_uuid {
        if !parent_uuid_str.is_empty() {
            // If we have an expected path (from parent entity), validate it
            if let Some(expected) = &expected_path {
                if let Some(actual_path) = &path {
                    if actual_path != expected {
                        violations.push(FieldViolation {
                            field: "path".to_string(),
                            message: format!(
                                "Path must match parent's path + key. Expected: {expected}, got: {actual_path}"
                            ),
                        });
                    }
                } else {
                    violations.push(FieldViolation {
                        field: "path".to_string(),
                        message: "Path is required when parent_uuid is set".to_string(),
                    });
                }
            }
        }
    }

    Ok(violations)
}
