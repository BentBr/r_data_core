#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{post, web, HttpResponse, Responder};
use serde_json::json;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::CombinedRequiredAuth;
use r_data_core_core::public_api::AdvancedEntityQuery;
use r_data_core_persistence::DynamicEntityQueryRepository;

/// Advanced query for dynamic entities with more complex filtering
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
        (status = 401, description = "Unauthorized - No valid authentication provided"),
        (status = 404, description = "Entity type not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
#[post("/{entity_type}/query")]
pub async fn query_entities(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<String>,
    query: web::Json<AdvancedEntityQuery>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let entity_type = path.into_inner();
    let repository = DynamicEntityQueryRepository::new(data.db_pool().clone());

    match repository
        .query_entities(&entity_type, &query.into_inner())
        .await
    {
        Ok(entities) => HttpResponse::Ok().json(entities),
        Err(e) => match e {
            r_data_core_core::error::Error::NotFound(msg) => HttpResponse::NotFound().json(json!({
                "error": msg
            })),
            _ => HttpResponse::InternalServerError().json(json!({
                "error": format!("Server error: {e}")
            })),
        },
    }
}

/// Register query routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(query_entities);
}
