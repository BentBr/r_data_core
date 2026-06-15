#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::web;
use log::error;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::public::dynamic_entities::models::DynamicEntityResponse;
use crate::response::ApiResponse;
use actix_web::HttpResponse;
use r_data_core_core::DynamicEntity;

/// Convert `DynamicEntity` to `DynamicEntityResponse`
/// Cannot use From trait since `DynamicEntity` is from another crate
pub(super) fn to_dynamic_entity_response(entity: DynamicEntity) -> DynamicEntityResponse {
    DynamicEntityResponse {
        entity_type: entity.entity_type,
        field_data: entity.field_data,
        children_count: None,
    }
}

/// Convert `DynamicEntity` to `DynamicEntityResponse` with children count
pub(super) fn to_dynamic_entity_response_with_children_count(
    entity: DynamicEntity,
    children_count: Option<i64>,
) -> DynamicEntityResponse {
    DynamicEntityResponse {
        entity_type: entity.entity_type,
        field_data: entity.field_data,
        children_count,
    }
}

/// Helper to validate requested fields against entity definition
///
/// # Errors
///
/// Returns an `Err(HttpResponse)` with an appropriate error response if the entity definition
/// cannot be found or if any requested fields are invalid for the given entity type.
pub(super) async fn validate_requested_fields(
    data: &web::Data<ApiStateWrapper>,
    entity_type: &str,
    fields: Option<&Vec<String>>,
) -> Result<(), HttpResponse> {
    if let Some(fields) = fields {
        let entity_def_service = data.entity_definition_service();
        match entity_def_service
            .get_entity_definition_by_entity_type(entity_type)
            .await
        {
            Ok(entity_def) => {
                // Always include these system fields
                let system_fields = [
                    "uuid",
                    "created_at",
                    "updated_at",
                    "created_by",
                    "updated_by",
                    "published",
                    "version",
                    "path",
                ];

                // Validate the requested fields
                let invalid_fields: Vec<String> = fields
                    .iter()
                    .filter(|field| {
                        !system_fields.contains(&field.as_str())
                            && entity_def.get_field(field).is_none()
                    })
                    .cloned()
                    .collect();

                if !invalid_fields.is_empty() {
                    return Err(ApiResponse::<()>::unprocessable_entity(&format!(
                        "Invalid fields requested: {}",
                        invalid_fields.join(", ")
                    )));
                }
            }
            Err(e) => return Err(handle_entity_error(e, entity_type)),
        }
    }
    Ok(())
}

/// Extract field name from unique violation message
/// Message format: "Field '`field_name`' must be unique..."
pub(super) fn extract_field_from_unique_message(msg: &str) -> String {
    if let Some(start) = msg.find("Field '") {
        let rest = &msg[start + 7..];
        if let Some(end) = rest.find('\'') {
            return rest[..end].to_string();
        }
    }
    "unknown".to_string()
}

/// Helper function to handle entity-related errors
pub(super) fn handle_entity_error(
    error: r_data_core_core::error::Error,
    entity_type: &str,
) -> HttpResponse {
    match error {
        r_data_core_core::error::Error::NotFound(_) => ApiResponse::<()>::not_found(&format!(
            "Entity type '{entity_type}' not found or not published"
        )),
        r_data_core_core::error::Error::Validation(msg) => {
            ApiResponse::<()>::unprocessable_entity(&msg)
        }
        r_data_core_core::error::Error::Database(_) => {
            error!("Database error: {error}");
            ApiResponse::<()>::internal_error("Database error")
        }
        _ => {
            error!("Internal error: {error}");
            ApiResponse::<()>::internal_error("Internal server error")
        }
    }
}
