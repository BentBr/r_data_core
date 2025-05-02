use actix_web::{get, web, HttpResponse, Responder};
use serde_json::json;

use super::repository::EntityRepository;
use crate::api::auth::auth_enum::CombinedRequiredAuth;
use crate::api::ApiState;

/// List all available entity types
#[utoipa::path(
    get,
    path = "/api/v1/entities",
    tag = "public",
    responses(
        (status = 200, description = "List of available entities", body = Vec<EntityTypeInfo>),
        (status = 401, description = "Unauthorized - No valid authentication provided"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
#[get("/entities")]
async fn list_available_entities(
    data: web::Data<ApiState>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let repository = EntityRepository::new(data.db_pool.clone());

    match repository.list_available_entities().await {
        Ok(entities) => HttpResponse::Ok().json(entities),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to list entities: {}", e)
        })),
    }
}

/// Register entity routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_available_entities);

    // Additional routes would be added here
    // For full implementation, create_entity, update_entity, delete_entity, etc.
}
