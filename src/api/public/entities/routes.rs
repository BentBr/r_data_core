use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use serde_json::json;
use uuid::Uuid;

use super::models::{EntityQuery, EntityTypeInfo};
use super::repository::EntityRepository;
use crate::api::ApiState;
use crate::entity::DynamicEntity;

/// List all available entity types
#[utoipa::path(
    get,
    path = "/api/v1/entities",
    tag = "public",
    responses(
        (status = 200, description = "List of available entities", body = Vec<EntityTypeInfo>),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/entities")]
async fn list_available_entities(data: web::Data<ApiState>) -> impl Responder {
    let repository = EntityRepository::new(data.db_pool.clone());

    match repository.list_available_entities().await {
        Ok(entities) => HttpResponse::Ok().json(entities),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to list entities: {}", e)
        })),
    }
}

/// Get a specific entity by type and UUID
#[utoipa::path(
    get,
    path = "/api/v1/{entity_type}/{uuid}",
    tag = "public",
    params(
        ("entity_type" = String, Path, description = "Entity type"),
        ("uuid" = Uuid, Path, description = "Entity UUID")
    ),
    responses(
        (status = 200, description = "Entity found", body = DynamicEntity),
        (status = 404, description = "Entity not found"),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/{entity_type}/{uuid}")]
async fn get_entity(data: web::Data<ApiState>, path: web::Path<(String, Uuid)>) -> impl Responder {
    let (entity_type, uuid) = path.into_inner();
    let repository = EntityRepository::new(data.db_pool.clone());

    match repository.get_entity(&entity_type, &uuid).await {
        Ok(entity) => HttpResponse::Ok().json(entity),
        Err(e) => match e {
            crate::error::Error::NotFound(msg) => HttpResponse::NotFound().json(json!({
                "error": msg
            })),
            _ => HttpResponse::InternalServerError().json(json!({
                "error": format!("Server error: {}", e)
            })),
        },
    }
}

/// Register entity routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_available_entities).service(get_entity);

    // Additional routes would be added here
    // For full implementation, create_entity, update_entity, delete_entity, etc.
}
