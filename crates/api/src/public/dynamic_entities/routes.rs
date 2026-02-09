use actix_web::{web, HttpResponse};
use log::{error, info};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::CombinedRequiredAuth;
use crate::query::StandardQuery;
use crate::response::{ApiResponse, ValidationViolation};
use r_data_core_core::domain::dynamic_entity::validator::{
    validate_entity_with_violations, FieldViolation,
};
use r_data_core_core::DynamicEntity;

/// Register routes for dynamic entities
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    info!("Registering dynamic entity routes");
    cfg.service(
        web::scope("")
            .route("/{entity_type}", web::get().to(list_entities))
            .route("/{entity_type}", web::post().to(create_entity))
            .route("/{entity_type}/{uuid}", web::get().to(get_entity))
            .route("/{entity_type}/{uuid}", web::put().to(update_entity))
            .route("/{entity_type}/{uuid}", web::delete().to(delete_entity)),
    );
}

use crate::public::dynamic_entities::models::{DynamicEntityResponse, EntityResponse};

// Helper function to convert DynamicEntity to DynamicEntityResponse
// Cannot use From trait since DynamicEntity is from another crate
fn to_dynamic_entity_response(entity: DynamicEntity) -> DynamicEntityResponse {
    DynamicEntityResponse {
        entity_type: entity.entity_type,
        field_data: entity.field_data,
        children_count: None,
    }
}

// Helper function to convert DynamicEntity to DynamicEntityResponse with children count
fn to_dynamic_entity_response_with_children_count(
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
async fn validate_requested_fields(
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

/// List entities of a specific type with pagination and filtering
#[utoipa::path(
    get,
    path = "/api/v1/{entity_type}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "Type of entity to list"),
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Number of items per page (default: 20, max: 100)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return (alternative to per_page)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (alternative to page-based pagination)"),
        ("include" = Option<String>, Query, description = "Comma-separated list of related entities to include"),
        ("sort_by" = Option<String>, Query, description = "Field to sort by"),
        ("sort_order" = Option<String>, Query, description = "Sort order: 'asc' or 'desc' (default: 'asc')"),
        ("fields" = Option<Vec<String>>, Query, description = "Fields to include in the response"),
        ("filter" = Option<HashMap<String, Value>>, Query, description = "Filter criteria")
    ),
    responses(
        (status = 200, description = "List of entities with pagination", body = Vec<DynamicEntityResponse>),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Entity type not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
pub async fn list_entities(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<String>,
    query: web::Query<StandardQuery>,
    _: CombinedRequiredAuth,
) -> HttpResponse {
    let entity_type = path.into_inner();
    let (limit, offset) = query.pagination.to_limit_offset(20, 100);
    let fields = query.fields.get_fields();
    let sort_by = query.sorting.sort_by.clone();
    let sort_direction = Some(query.sorting.get_sort_order());

    // Handle filters and also accept a "path" query param for folder-style browsing
    let filter = query.filter.parse_filter();
    // Also honor a "folder" shorthand via sorting.sort_by when set to "path" or explicit path in query.q with prefix "path:"
    // Prefer JSON filter {"path": "/..."} from clients.
    let search_query = query.filter.q.clone();

    // Validate requested fields
    if let Err(response) = validate_requested_fields(&data, &entity_type, fields.as_ref()).await {
        return response;
    }

    if let Some(service) = data.dynamic_entity_service() {
        // If validation passed, proceed with the query
        match service
            .list_entities_with_filters(
                &entity_type,
                limit,
                offset,
                fields,
                sort_by,
                sort_direction,
                filter,
                search_query,
            )
            .await
        {
            Ok((entities, total)) => {
                let entity_responses: Vec<DynamicEntityResponse> = entities
                    .into_iter()
                    .map(to_dynamic_entity_response)
                    .collect();

                let page = query.pagination.get_page(1);
                let per_page = query.pagination.get_per_page(20, 100);

                ApiResponse::ok_paginated(entity_responses, total, page, per_page)
            }
            Err(e) => handle_entity_error(e, &entity_type),
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

/// Handler for creating a new entity
#[utoipa::path(
    post,
    path = "/api/v1/{entity_type}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "The type of entity to create")
    ),
    request_body = HashMap<String, Value>,
    responses(
        (status = 201, description = "Entity created successfully", body = EntityResponse),
        (status = 400, description = "Invalid entity data"),
        (status = 404, description = "Entity type not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
#[allow(clippy::implicit_hasher)] // Actix Web extractor requires concrete HashMap
pub async fn create_entity(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<String>,
    entity: web::Json<HashMap<String, Value>>,
    auth: CombinedRequiredAuth,
) -> HttpResponse {
    let entity_type = path.into_inner();

    // Get the user's UUID from either API key or JWT
    let Some(user_uuid) = auth.get_user_uuid() else {
        return ApiResponse::<()>::unauthorized(
            "User UUID could not be determined from authentication",
        );
    };

    if let Some(service) = data.dynamic_entity_service() {
        // First, we need to find the entity definition to create the entity
        let entity_def_service = data.entity_definition_service();
        match entity_def_service
            .get_entity_definition_by_entity_type(&entity_type)
            .await
        {
            Ok(entity_def) => {
                if !entity_def.published {
                    return ApiResponse::<()>::not_found(&format!(
                        "Entity type {entity_type} not found or not published"
                    ));
                }

                // We need to create a dynamic entity
                let mut field_data = entity.into_inner();
                field_data.insert("created_by".to_string(), json!(user_uuid.to_string()));
                field_data.insert("updated_by".to_string(), json!(user_uuid.to_string()));

                // Validate entity before creation
                let entity_json = json!({
                    "entity_type": entity_type,
                    "field_data": field_data
                });

                let validation_result: Result<Vec<FieldViolation>, _> =
                    validate_entity_with_violations(&entity_json, &entity_def);
                match validation_result {
                    Ok(ref violations) if !violations.is_empty() => {
                        // Convert to Symfony-style violations
                        let violations: Vec<ValidationViolation> = violations
                            .iter()
                            .map(|v| ValidationViolation {
                                field: v.field.clone(),
                                message: v.message.clone(),
                                code: Some("INVALID".to_string()),
                            })
                            .collect();
                        return ApiResponse::unprocessable_entity_with_violations(
                            "Validation failed",
                            violations,
                        );
                    }
                    Err(e) => return handle_entity_error(e, &entity_type),
                    _ => {} // Validation passed
                }

                let dynamic_entity = DynamicEntity {
                    entity_type: entity_type.clone(),
                    field_data,
                    definition: std::sync::Arc::new(entity_def),
                };

                match service.create_entity(&dynamic_entity).await {
                    Ok(uuid) => {
                        let response_data = EntityResponse { uuid, entity_type };
                        ApiResponse::<EntityResponse>::created(response_data)
                    }
                    Err(e) => {
                        // Map unique violation to appropriate error response
                        if let r_data_core_core::error::Error::ValidationFailed(msg) = &e {
                            if msg.contains("same key") {
                                return ApiResponse::<()>::conflict(msg);
                            }
                            // Handle unique field constraint violations
                            if msg.contains("must be unique") {
                                let field = extract_field_from_unique_message(msg);
                                let violations = vec![crate::response::ValidationViolation {
                                    field,
                                    message: msg.clone(),
                                    code: Some("UNIQUE_VIOLATION".to_string()),
                                }];
                                return ApiResponse::<()>::unprocessable_entity_with_violations(
                                    "Validation failed",
                                    violations,
                                );
                            }
                        }
                        handle_entity_error(e, &entity_type)
                    }
                }
            }
            Err(e) => handle_entity_error(e, &entity_type),
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

/// Get a specific entity by UUID
#[utoipa::path(
    get,
    path = "/api/v1/{entity_type}/{uuid}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "Type of entity"),
        ("uuid" = Uuid, Path, description = "Entity UUID"),
        ("include" = Option<String>, Query, description = "Comma-separated list of related entities to include"),
        ("include_children_count" = Option<bool>, Query, description = "Include count of child entities"),
        ("fields" = Option<Vec<String>>, Query, description = "Fields to include in the response")
    ),
    responses(
        (status = 200, description = "Entity found", body = DynamicEntityResponse),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Entity not found"),
        (status = 422, description = "Invalid field requested"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
pub async fn get_entity(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<(String, String)>,
    query: web::Query<StandardQuery>,
    _: CombinedRequiredAuth,
) -> HttpResponse {
    let (entity_type, uuid_str) = path.into_inner();
    let fields = query.fields.get_fields();
    let _includes = query.include.get_includes();
    let include_children_count = query.include.should_include_children_count();

    // Validate requested fields
    if let Err(response) = validate_requested_fields(&data, &entity_type, fields.as_ref()).await {
        return response;
    }

    // Parse UUID
    let Ok(uuid) = Uuid::parse_str(&uuid_str) else {
        return ApiResponse::<()>::bad_request(&format!("Invalid UUID format: {uuid_str}"));
    };

    if let Some(service) = data.dynamic_entity_service() {
        match service
            .get_entity_by_uuid_with_children_count(
                &entity_type,
                &uuid,
                fields,
                include_children_count,
            )
            .await
        {
            Ok((Some(entity), children_count)) => {
                let response =
                    to_dynamic_entity_response_with_children_count(entity, children_count);
                ApiResponse::ok(response)
            }
            Ok((None, _)) => ApiResponse::<()>::not_found(&format!(
                "Entity of type '{entity_type}' with UUID '{uuid}' not found"
            )),
            Err(e) => handle_entity_error(e, &entity_type),
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

/// Handler for updating an entity
#[utoipa::path(
    put,
    path = "/api/v1/{entity_type}/{uuid}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "The type of entity to update"),
        ("uuid" = uuid::Uuid, Path, description = "The UUID of the entity to update")
    ),
    request_body = HashMap<String, Value>,
    responses(
        (status = 200, description = "Entity updated successfully", body = EntityResponse),
        (status = 400, description = "Invalid entity data"),
        (status = 404, description = "Entity or field not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
#[allow(clippy::implicit_hasher)] // Actix Web extractor requires concrete HashMap
pub async fn update_entity(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<(String, String)>,
    entity_data: web::Json<HashMap<String, Value>>,
    auth: CombinedRequiredAuth,
) -> HttpResponse {
    let (entity_type, uuid_str) = path.into_inner();
    let Ok(uuid) = Uuid::parse_str(&uuid_str) else {
        return ApiResponse::<()>::bad_request(&format!("Invalid UUID: {uuid_str}"));
    };

    // Get the user's UUID
    let Some(user_uuid) = auth.get_user_uuid() else {
        return ApiResponse::<()>::unauthorized(
            "User UUID could not be determined from authentication",
        );
    };

    if let Some(service) = data.dynamic_entity_service() {
        // First, we need to get the existing entity
        match service.get_entity_by_uuid(&entity_type, &uuid, None).await {
            Ok(Some(mut existing_entity)) => {
                // Update the entity with the new data
                let mut new_data = entity_data.into_inner();

                // Ensure UUID is consistent
                new_data.insert("uuid".to_string(), json!(uuid.to_string()));

                // Add audit fields
                new_data.insert("updated_by".to_string(), json!(user_uuid.to_string()));

                // Merge the new data with existing data (update only changed fields)
                for (key, value) in new_data {
                    existing_entity.field_data.insert(key, value);
                }

                match service.update_entity(&existing_entity).await {
                    Ok(()) => {
                        let response_data = EntityResponse { uuid, entity_type };
                        ApiResponse::ok(response_data)
                    }
                    Err(e) => {
                        if let r_data_core_core::error::Error::ValidationFailed(msg) = &e {
                            if msg.contains("same key") {
                                return ApiResponse::<()>::conflict(msg);
                            }
                            // Handle unique field constraint violations
                            if msg.contains("must be unique") {
                                let field = extract_field_from_unique_message(msg);
                                let violations = vec![crate::response::ValidationViolation {
                                    field,
                                    message: msg.clone(),
                                    code: Some("UNIQUE_VIOLATION".to_string()),
                                }];
                                return ApiResponse::<()>::unprocessable_entity_with_violations(
                                    "Validation failed",
                                    violations,
                                );
                            }
                        }
                        handle_entity_error(e, &entity_type)
                    }
                }
            }
            Ok(None) => ApiResponse::<()>::not_found(&format!(
                "Entity with UUID {uuid} not found in type {entity_type}"
            )),
            Err(e) => handle_entity_error(e, &entity_type),
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

/// Handler for deleting an entity
#[utoipa::path(
    delete,
    path = "/api/v1/{entity_type}/{uuid}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "The type of entity to delete"),
        ("uuid" = uuid::Uuid, Path, description = "The UUID of the entity to delete")
    ),
    responses(
        (status = 200, description = "Entity deleted successfully"),
        (status = 404, description = "Entity not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
pub async fn delete_entity(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<(String, String)>,
    _: CombinedRequiredAuth,
) -> HttpResponse {
    let (entity_type, uuid_str) = path.into_inner();
    let Ok(uuid) = Uuid::parse_str(&uuid_str) else {
        return ApiResponse::<()>::bad_request(&format!("Invalid UUID: {uuid_str}"));
    };

    if let Some(service) = data.dynamic_entity_service() {
        match service.delete_entity(&entity_type, &uuid).await {
            Ok(()) => ApiResponse::<()>::message("Successfully deleted the entity"),
            Err(e) => handle_entity_error(e, &entity_type),
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

/// Extract field name from unique violation message
/// Message format: "Field '`field_name`' must be unique..."
fn extract_field_from_unique_message(msg: &str) -> String {
    if let Some(start) = msg.find("Field '") {
        let rest = &msg[start + 7..];
        if let Some(end) = rest.find('\'') {
            return rest[..end].to_string();
        }
    }
    "unknown".to_string()
}

/// Helper function to handle entity-related errors
fn handle_entity_error(error: r_data_core_core::error::Error, entity_type: &str) -> HttpResponse {
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
