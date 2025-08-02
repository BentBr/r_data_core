use actix_web::{web, HttpResponse};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::auth::auth_enum::CombinedRequiredAuth;
use crate::api::middleware::ApiKeyInfo;
use crate::api::query::StandardQuery;
use crate::api::response::ApiResponse;
use crate::api::ApiState;
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::error::Error;

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

/// Schema for dynamic entity serialization
#[derive(Serialize, Deserialize, ToSchema)]
pub struct DynamicEntityResponse {
    pub entity_type: String,
    pub field_data: HashMap<String, Value>,
}

impl From<DynamicEntity> for DynamicEntityResponse {
    fn from(entity: DynamicEntity) -> Self {
        Self {
            entity_type: entity.entity_type,
            field_data: entity.field_data,
        }
    }
}

/// Response for entity creation/update
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EntityResponse {
    pub uuid: Uuid,
    pub entity_type: String,
}

/// Helper to validate requested fields against entity definition
async fn validate_requested_fields(
    data: &web::Data<ApiState>,
    entity_type: &str,
    fields: &Option<Vec<String>>,
) -> Result<(), HttpResponse> {
    if let Some(fields) = fields {
        let entity_def_service = &data.entity_definition_service;
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

/// Handler for listing entities of a specific type
#[utoipa::path(
    get,
    path = "/api/v1/{entity_type}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "The type of entity to list"),
        ("page" = Option<i64>, Query, description = "Page number (1-based)"),
        ("per_page" = Option<i64>, Query, description = "Number of items per page"),
        ("sort_by" = Option<String>, Query, description = "Field to sort by"),
        ("sort_direction" = Option<String>, Query, description = "Sort direction (ASC or DESC)"),
        ("fields" = Option<String>, Query, description = "Comma-separated list of fields to include"),
        ("filter" = Option<String>, Query, description = "Filter expression"),
        ("q" = Option<String>, Query, description = "Search query"),
        ("include" = Option<String>, Query, description = "Comma-separated list of related entities to include")
    ),
    responses(
        (status = 200, description = "List of entities", body = Vec<DynamicEntityResponse>),
        (status = 404, description = "Entity type not found"),
        (status = 422, description = "Invalid field requested"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
async fn list_entities(
    data: web::Data<ApiState>,
    path: web::Path<String>,
    query: web::Query<StandardQuery>,
    _: CombinedRequiredAuth,
) -> HttpResponse {
    let entity_type = path.into_inner();
    let (limit, offset) = query.pagination.to_limit_offset(1, 20, 100);
    let fields = query.fields.get_fields();
    let sort_by = query.sorting.sort_by.clone();
    let sort_direction = Some(query.sorting.get_sort_direction());

    // Handle filters - we'll need to adjust the service to handle the new filter format
    let filter = query.filter.parse_filter();
    let search_query = query.filter.q.clone();

    // Validate requested fields
    if let Err(response) = validate_requested_fields(&data, &entity_type, &fields).await {
        return response;
    }

    if let Some(service) = &data.dynamic_entity_service {
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
                    .map(DynamicEntityResponse::from)
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
async fn create_entity(
    data: web::Data<ApiState>,
    path: web::Path<String>,
    entity: web::Json<HashMap<String, Value>>,
    auth: CombinedRequiredAuth,
) -> HttpResponse {
    let entity_type = path.into_inner();

    // Get the user's UUID from either API key or JWT
    let user_uuid = match auth.get_user_uuid() {
        Some(uuid) => uuid,
        None => {
            return ApiResponse::<()>::unauthorized(
                "User UUID could not be determined from authentication",
            );
        }
    };

    if let Some(service) = &data.dynamic_entity_service {
        // First, we need to find the entity definition to create the entity
        let entity_def_service = &data.entity_definition_service;
        match entity_def_service
            .get_entity_definition_by_entity_type(&entity_type)
            .await
        {
            Ok(entity_def) => {
                if !entity_def.published {
                    return ApiResponse::<()>::not_found(&format!(
                        "Entity type {} not found or not published",
                        entity_type
                    ));
                }

                // We need to create a dynamic entity
                let uuid = Uuid::now_v7();
                let mut field_data = entity.into_inner();
                field_data.insert("uuid".to_string(), json!(uuid.to_string()));
                field_data.insert("created_by".to_string(), json!(user_uuid.to_string()));
                field_data.insert("updated_by".to_string(), json!(user_uuid.to_string()));

                let dynamic_entity = DynamicEntity {
                    entity_type: entity_type.clone(),
                    field_data,
                    definition: std::sync::Arc::new(entity_def),
                };

                match service.create_entity(&dynamic_entity).await {
                    Ok(_) => {
                        let response_data = EntityResponse { uuid, entity_type };
                        ApiResponse::<EntityResponse>::created(response_data)
                    }
                    Err(e) => handle_entity_error(e, &entity_type),
                }
            }
            Err(e) => handle_entity_error(e, &entity_type),
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

/// Handler for getting a single entity by UUID
#[utoipa::path(
    get,
    path = "/api/v1/{entity_type}/{uuid}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "The type of entity to get"),
        ("uuid" = Uuid, Path, description = "The UUID of the entity to get"),
        ("fields" = Option<String>, Query, description = "Comma-separated list of fields to include"),
        ("include" = Option<String>, Query, description = "Comma-separated list of related entities to include")
    ),
    responses(
        (status = 200, description = "Entity retrieved successfully", body = DynamicEntityResponse),
        (status = 404, description = "Entity not found"),
        (status = 422, description = "Invalid field requested"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
async fn get_entity(
    data: web::Data<ApiState>,
    path: web::Path<(String, String)>,
    query: web::Query<StandardQuery>,
    _: CombinedRequiredAuth,
) -> HttpResponse {
    let (entity_type, uuid_str) = path.into_inner();
    let fields = query.fields.get_fields();
    let includes = query.include.get_includes();

    // Validate requested fields
    if let Err(response) = validate_requested_fields(&data, &entity_type, &fields).await {
        return response;
    }

    // Parse UUID
    let uuid = match Uuid::parse_str(&uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => {
            return ApiResponse::<()>::bad_request(&format!("Invalid UUID format: {}", uuid_str));
        }
    };

    if let Some(service) = &data.dynamic_entity_service {
        match service
            .get_entity_by_uuid(&entity_type, &uuid, fields)
            .await
        {
            Ok(Some(entity)) => {
                let response = DynamicEntityResponse::from(entity);
                ApiResponse::ok(response)
            }
            Ok(None) => ApiResponse::<()>::not_found(&format!(
                "Entity of type '{}' with UUID '{}' not found",
                entity_type, uuid
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
async fn update_entity(
    data: web::Data<ApiState>,
    path: web::Path<(String, String)>,
    entity_data: web::Json<HashMap<String, Value>>,
    auth: CombinedRequiredAuth,
) -> HttpResponse {
    let (entity_type, uuid_str) = path.into_inner();
    let uuid = match Uuid::parse_str(&uuid_str) {
        Ok(id) => id,
        Err(_) => {
            return ApiResponse::<()>::bad_request(&format!("Invalid UUID: {}", uuid_str));
        }
    };

    // Get the user's UUID
    let user_uuid = match auth.get_user_uuid() {
        Some(id) => id,
        None => {
            return ApiResponse::<()>::unauthorized(
                "User UUID could not be determined from authentication",
            );
        }
    };

    if let Some(service) = &data.dynamic_entity_service {
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
                    Ok(_) => {
                        let response_data = EntityResponse { uuid, entity_type };
                        ApiResponse::ok(response_data)
                    }
                    Err(e) => handle_entity_error(e, &entity_type),
                }
            }
            Ok(None) => ApiResponse::<()>::not_found(&format!(
                "Entity with UUID {} not found in type {}",
                uuid, entity_type
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
        (status = 204, description = "Entity deleted successfully"),
        (status = 404, description = "Entity not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
async fn delete_entity(
    data: web::Data<ApiState>,
    path: web::Path<(String, String)>,
    _: CombinedRequiredAuth,
) -> HttpResponse {
    let (entity_type, uuid_str) = path.into_inner();
    let uuid = match Uuid::parse_str(&uuid_str) {
        Ok(id) => id,
        Err(_) => {
            return ApiResponse::<()>::bad_request(&format!("Invalid UUID: {}", uuid_str));
        }
    };

    if let Some(service) = &data.dynamic_entity_service {
        match service.delete_entity(&entity_type, &uuid).await {
            Ok(_) => HttpResponse::NoContent().finish(),
            Err(e) => handle_entity_error(e, &entity_type),
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

/// Helper function to handle entity-related errors
fn handle_entity_error(error: Error, entity_type: &str) -> HttpResponse {
    match error {
        Error::NotFound(_) => ApiResponse::<()>::not_found(&format!(
            "Entity type '{}' not found or not published",
            entity_type
        )),
        Error::Validation(msg) => ApiResponse::<()>::unprocessable_entity(&msg),
        Error::Database(_) => {
            error!("Database error: {}", error);
            ApiResponse::<()>::internal_error("Database error")
        }
        _ => {
            error!("Internal error: {}", error);
            ApiResponse::<()>::internal_error("Internal server error")
        }
    }
}
