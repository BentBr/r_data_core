use actix_web::{web, HttpResponse};
use log::{info, error};
use crate::api::ApiState;
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::error::Error;
use serde_json::{json, Value};
use uuid::Uuid;
use crate::api::response::ApiResponse;
use crate::api::auth::auth_enum::CombinedRequiredAuth;
use crate::api::middleware::ApiKeyInfo;

/// Register routes for dynamic entities
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    info!("Registering dynamic entity routes");
    // Get a list of entity types at startup and register routes for each
    cfg.service(
        web::scope("")
            .route("/{entity_type}", web::get().to(list_entities))
            .route("/{entity_type}", web::post().to(create_entity))
            .route("/{entity_type}/{uuid}", web::get().to(get_entity))
            .route("/{entity_type}/{uuid}", web::put().to(update_entity))
            .route("/{entity_type}/{uuid}", web::delete().to(delete_entity))
    );
}

/// Handler for listing entities of a specific type
#[utoipa::path(
    get,
    path = "/api/v1/{entity_type}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "The type of entity to list"),
        ("limit" = Option<i64>, Query, description = "Maximum number of entities to return"),
        ("offset" = Option<i64>, Query, description = "Number of entities to skip")
    ),
    responses(
        (status = 200, description = "List of entities", body = Vec<DynamicEntity>),
        (status = 404, description = "Entity type not found"),
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
    query: web::Query<PaginationQuery>,
    _: CombinedRequiredAuth,
) -> HttpResponse {
    let entity_type = path.into_inner();
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    if let Some(service) = &data.dynamic_entity_service {
        match service.list_entities(&entity_type, limit, offset).await {
            Ok(entities) => ApiResponse::ok(entities),
            Err(e) => handle_entity_error(e, &entity_type)
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
    request_body = HashMap<String, serde_json::Value>,
    responses(
        (status = 201, description = "Entity created successfully", body = Value),
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
    entity: web::Json<HashMap<String, serde_json::Value>>,
    auth: CombinedRequiredAuth,
) -> HttpResponse {
    let entity_type = path.into_inner();
    
    // Get the user's UUID from either API key or JWT
    let user_uuid = match auth.get_user_uuid() {
        Some(uuid) => uuid,
        None => {
            return ApiResponse::<()>::unauthorized("User UUID could not be determined from authentication");
        }
    };
    
    if let Some(service) = &data.dynamic_entity_service {
        // First, we need to find the class definition to create the entity
        let class_def_service = &data.class_definition_service;
        match class_def_service.get_class_definition_by_entity_type(&entity_type).await {
            Ok(class_def) => {
                // We need to create a dynamic entity
                let uuid = Uuid::now_v7();
                let mut field_data = entity.into_inner();
                field_data.insert("uuid".to_string(), json!(uuid.to_string()));
                field_data.insert("created_by".to_string(), json!(user_uuid.to_string()));
                field_data.insert("updated_by".to_string(), json!(user_uuid.to_string()));
                
                let dynamic_entity = DynamicEntity {
                    entity_type: entity_type.clone(),
                    field_data,
                    definition: Arc::new(class_def),
                };
                
                match service.create_entity(&dynamic_entity).await {
                    Ok(_) => {
                        let response_data: Value = json!({
                            "uuid": uuid,
                            "entity_type": entity_type
                        });
                        ApiResponse::<Value>::created(response_data)
                    },
                    Err(e) => handle_entity_error(e, &entity_type)
                }
            },
            Err(e) => handle_entity_error(e, &entity_type)
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

/// Handler for getting a specific entity by UUID
#[utoipa::path(
    get,
    path = "/api/v1/{entity_type}/{uuid}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "The type of entity to retrieve"),
        ("uuid" = uuid::Uuid, Path, description = "The UUID of the entity to retrieve")
    ),
    responses(
        (status = 200, description = "Entity found", body = DynamicEntity),
        (status = 404, description = "Entity not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
async fn get_entity(
    data: web::Data<ApiState>,
    path: web::Path<(String, uuid::Uuid)>,
    auth: CombinedRequiredAuth,
) -> HttpResponse {
    let (entity_type, uuid) = path.into_inner();
    
    if let Some(service) = &data.dynamic_entity_service {
        match service.get_entity_by_uuid(&entity_type, &uuid).await {
            Ok(Some(entity)) => ApiResponse::ok(entity),
            Ok(None) => ApiResponse::<()>::not_found(&format!("{} with UUID {}", entity_type, uuid)),
            Err(e) => handle_entity_error(e, &entity_type)
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

/// Handler for updating an existing entity
#[utoipa::path(
    put,
    path = "/api/v1/{entity_type}/{uuid}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "The type of entity to update"),
        ("uuid" = uuid::Uuid, Path, description = "The UUID of the entity to update")
    ),
    request_body = HashMap<String, serde_json::Value>,
    responses(
        (status = 200, description = "Entity updated successfully", body = Value),
        (status = 400, description = "Invalid entity data"),
        (status = 404, description = "Entity not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
async fn update_entity(
    data: web::Data<ApiState>,
    path: web::Path<(String, uuid::Uuid)>,
    entity_data: web::Json<HashMap<String, serde_json::Value>>,
    auth: CombinedRequiredAuth,
) -> HttpResponse {
    let (entity_type, uuid) = path.into_inner();
    
    // Get the user's UUID from either API key or JWT
    let user_uuid = match auth.get_user_uuid() {
        Some(uuid) => uuid,
        None => {
            return ApiResponse::<()>::unauthorized("User UUID could not be determined from authentication");
        }
    };
    
    if let Some(service) = &data.dynamic_entity_service {
        // First get the existing entity to update
        match service.get_entity_by_uuid(&entity_type, &uuid).await {
            Ok(Some(existing_entity)) => {
                // Now get the class definition
                let class_def_service = &data.class_definition_service;
                match class_def_service.get_class_definition_by_entity_type(&entity_type).await {
                    Ok(class_def) => {
                        // Create updated entity
                        let mut field_data = entity_data.into_inner();
                        field_data.insert("uuid".to_string(), json!(uuid.to_string()));
                        field_data.insert("updated_by".to_string(), json!(user_uuid.to_string()));
                        
                        // Keep created_by from existing entity if available
                        if let Some(created_by) = existing_entity.field_data.get("created_by") {
                            field_data.insert("created_by".to_string(), created_by.clone());
                        }
                        
                        let dynamic_entity = DynamicEntity {
                            entity_type: entity_type.clone(),
                            field_data,
                            definition: Arc::new(class_def),
                        };
                        
                        match service.update_entity(&dynamic_entity).await {
                            Ok(_) => {
                                let response_data: Value = json!({
                                    "uuid": uuid,
                                    "entity_type": entity_type
                                });
                                ApiResponse::<Value>::ok_with_message(response_data, &format!("{} updated successfully", entity_type))
                            },
                            Err(e) => handle_entity_error(e, &entity_type)
                        }
                    },
                    Err(e) => handle_entity_error(e, &entity_type)
                }
            },
            Ok(None) => ApiResponse::<()>::not_found(&format!("{} with UUID {}", entity_type, uuid)),
            Err(e) => handle_entity_error(e, &entity_type)
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
        (status = 200, description = "Entity deleted successfully", body = Value),
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
    path: web::Path<(String, uuid::Uuid)>,
    auth: CombinedRequiredAuth,
) -> HttpResponse {
    let (entity_type, uuid) = path.into_inner();
    
    if let Some(service) = &data.dynamic_entity_service {
        match service.delete_entity(&entity_type, &uuid).await {
            Ok(_) => {
                let response_data: Value = json!({
                    "uuid": uuid,
                    "entity_type": entity_type
                });
                ApiResponse::<Value>::ok_with_message(response_data, &format!("{} deleted successfully", entity_type))
            },
            Err(e) => handle_entity_error(e, &entity_type)
        }
    } else {
        ApiResponse::<()>::internal_error("Dynamic entity service not initialized")
    }
}

// Helper functions for responses and entity creation
/// Pagination query parameters
#[derive(serde::Deserialize, ToSchema)]
pub struct PaginationQuery {
    /// Maximum number of entities to return
    pub limit: Option<i64>,
    /// Number of entities to skip
    pub offset: Option<i64>,
}

/// Handles errors from the dynamic entity service
fn handle_entity_error(error: Error, entity_type: &str) -> HttpResponse {
    match error {
        Error::NotFound(_) => ApiResponse::<()>::not_found(entity_type),
        Error::Validation(msg) => ApiResponse::<()>::unprocessable_entity(&msg),
        e => {
            error!("Dynamic entity error: {}", e);
            ApiResponse::<()>::internal_error(&format!("Internal server error: {}", e))
        }
    }
} 