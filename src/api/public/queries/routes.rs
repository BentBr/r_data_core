use actix_web::{post, web, HttpResponse, Responder};
use serde_json::json;

use super::models::AdvancedEntityQuery;
use super::repository::QueryRepository;
use crate::api::ApiState;

/// Advanced query for entities with more complex filtering
#[utoipa::path(
    post,
    path = "/api/v1/{entity_type}/query",
    tag = "public",
    params(
        ("entity_type" = String, Path, description = "Entity type to query")
    ),
    request_body = AdvancedEntityQuery,
    responses(
        (status = 200, description = "Query results", body = Vec<DynamicEntity>),
        (status = 404, description = "Entity type not found"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/{entity_type}/query")]
async fn query_entities(
    data: web::Data<ApiState>,
    path: web::Path<String>,
    query: web::Json<AdvancedEntityQuery>,
) -> impl Responder {
    let entity_type = path.into_inner();
    let repository = QueryRepository::new(data.db_pool.clone());

    match repository.query_entities(&entity_type, &query).await {
        Ok(entities) => HttpResponse::Ok().json(entities),
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

/// Register query routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(query_entities);
}
