use actix_web::{get, post, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use super::models::BrowseNode;
use super::repository::EntityRepository;
use crate::api::auth::auth_enum::CombinedRequiredAuth;
use crate::api::response::ApiResponse;
use crate::api::ApiState;
use crate::entity::dynamic_entity::repository::DynamicEntityRepository;

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
    cfg.service(list_by_path);
    cfg.service(query_entities);
}

#[derive(Debug, Deserialize)]
struct BrowseQuery {
    /// Folder path to browse; defaults to "/"
    path: Option<String>,
    /// Limit number of returned items (folders+files combined)
    limit: Option<i64>,
    /// Offset for pagination
    offset: Option<i64>,
}

/// Browse entities by virtual folder path
#[utoipa::path(
    get,
    path = "/api/v1/entities/by-path",
    tag = "public",
    params(
        ("path" = Option<String>, Query, description = "Folder path to browse, e.g. '/' or '/myFolder'"),
        ("limit" = Option<i64>, Query, description = "Max items per page (default 20)"),
        ("offset" = Option<i64>, Query, description = "Items to skip (default 0)")
    ),
    responses(
        (status = 200, description = "Browse result (folders first, then files)", body = Vec<BrowseNode>),
        (status = 401, description = "Unauthorized - No valid authentication provided"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
#[get("/entities/by-path")]
async fn list_by_path(
    data: web::Data<ApiState>,
    query: web::Query<BrowseQuery>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let repository = EntityRepository::new(data.db_pool.clone());
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    match repository
        .browse_by_path(
            &query.path.clone().unwrap_or_else(|| "/".to_string()),
            limit,
            offset,
        )
        .await
    {
        Ok((nodes, total)) => ApiResponse::<Vec<BrowseNode>>::ok_paginated(
            nodes,
            total,
            (offset / limit) as i64 + 1,
            limit,
        ),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "status": "Error",
            "message": format!("Server error: {}", e),
        })),
    }
}

/// Request body for querying entities
#[derive(Debug, Deserialize, ToSchema)]
struct EntityQueryRequest {
    /// Entity type to query
    entity_type: String,
    /// Filter by parent UUID
    parent_uuid: Option<Uuid>,
    /// Filter by exact path
    path: Option<String>,
    /// Maximum number of results (default: 20, max: 100)
    limit: Option<i64>,
    /// Number of results to skip (default: 0)
    offset: Option<i64>,
}

/// Query entities by parent or path
#[utoipa::path(
    post,
    path = "/api/v1/entities/query",
    tag = "public",
    request_body(
        description = "Query parameters for filtering entities",
        content_type = "application/json",
        content = EntityQueryRequest
    ),
    responses(
        (status = 200, description = "List of entities matching the query", body = Vec<DynamicEntityResponse>),
        (status = 401, description = "Unauthorized - No valid authentication provided"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
#[post("/entities/query")]
async fn query_entities(
    data: web::Data<ApiState>,
    body: web::Json<EntityQueryRequest>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let repository = DynamicEntityRepository::new(data.db_pool.clone());
    
    let limit = body.limit.unwrap_or(20).clamp(1, 100);
    let offset = body.offset.unwrap_or(0).max(0);

    match repository
        .query_by_parent_and_path(
            &body.entity_type,
            body.parent_uuid,
            body.path.as_deref(),
            limit,
            offset,
        )
        .await
    {
        Ok(entities) => {
            // Convert DynamicEntity to DynamicEntityResponse
            use crate::api::public::dynamic_entities::routes::DynamicEntityResponse;
            let responses: Vec<DynamicEntityResponse> = entities
                .into_iter()
                .map(DynamicEntityResponse::from)
                .collect();
            
            ApiResponse::ok(responses)
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "status": "Error",
            "message": format!("Server error: {}", e),
        })),
    }
}
