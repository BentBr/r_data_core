#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, post, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::CombinedRequiredAuth;
use crate::public::dynamic_entities::models::DynamicEntityResponse;
use crate::public::entities::models::{EntityQueryRequest, VersionMeta, VersionPayload};
use crate::response::ApiResponse;
#[allow(unused_imports)] // Used in utoipa attributes for OpenAPI docs
use r_data_core_core::public_api::{BrowseNode, EntityTypeInfo};
use r_data_core_persistence::DynamicEntityPublicRepository;
use r_data_core_persistence::DynamicEntityRepository;
use r_data_core_persistence::VersionRepository;
use r_data_core_services::VersionService;

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
pub async fn list_available_entities(
    data: web::Data<ApiStateWrapper>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let repository = DynamicEntityPublicRepository::new(data.db_pool().clone());

    match repository.list_available_entity_types().await {
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
    cfg.service(list_entity_versions);
    cfg.service(get_entity_version);
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
pub async fn list_by_path(
    data: web::Data<ApiStateWrapper>,
    query: web::Query<BrowseQuery>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let repository = DynamicEntityPublicRepository::new(data.db_pool().clone());
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
        Ok((nodes, total)) => {
            ApiResponse::ok_paginated(nodes, total, (offset / limit) as i64 + 1, limit)
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "status": "Error",
            "message": format!("Server error: {}", e),
        })),
    }
}

/// List versions of a dynamic entity
#[utoipa::path(
    get,
    path = "/api/v1/entities/{entity_type}/{uuid}/versions",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "Entity type"),
        ("uuid" = Uuid, Path, description = "Entity UUID")
    ),
    responses(
        (status = 200, description = "List of versions", body = Vec<VersionMeta>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Entity not found"),
        (status = 500, description = "Server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
#[get("/entities/{entity_type}/{uuid}/versions")]
pub async fn list_entity_versions(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<(String, Uuid)>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let (_entity_type, uuid) = path.into_inner();

    let version_service = VersionService::new(data.db_pool().clone());

    match version_service
        .list_entity_versions_with_metadata(uuid)
        .await
    {
        Ok(versions) => {
            let out: Vec<VersionMeta> = versions
                .into_iter()
                .map(|v| VersionMeta {
                    version_number: v.version_number,
                    created_at: v.created_at,
                    created_by: v.created_by,
                    created_by_name: v.created_by_name,
                })
                .collect();
            ApiResponse::ok(out)
        }
        Err(e) => {
            log::error!("Failed to list versions: {}", e);
            ApiResponse::<()>::internal_error("Failed to list versions")
        }
    }
}

/// Get a specific version snapshot of a dynamic entity
#[utoipa::path(
    get,
    path = "/api/v1/entities/{entity_type}/{uuid}/versions/{version_number}",
    tag = "dynamic-entities",
    params(
        ("entity_type" = String, Path, description = "Entity type"),
        ("uuid" = Uuid, Path, description = "Entity UUID"),
        ("version_number" = i32, Path, description = "Version number")
    ),
    responses(
        (status = 200, description = "Version payload", body = VersionPayload),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Version not found"),
        (status = 500, description = "Server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
#[get("/entities/{entity_type}/{uuid}/versions/{version_number}")]
pub async fn get_entity_version(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<(String, Uuid, i32)>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let (entity_type, uuid, version_number) = path.into_inner();

    let repo = VersionRepository::new(data.db_pool().clone());

    // First try to get from versions table
    match repo.get_entity_version(uuid, version_number).await {
        Ok(Some(row)) => {
            let payload = VersionPayload {
                version_number: row.version_number,
                created_at: row.created_at,
                created_by: row.created_by,
                data: row.data,
            };
            return ApiResponse::ok(payload);
        }
        Ok(None) => {
            // Not in versions table, check if it's the current version
            let current_metadata = repo.get_current_entity_metadata(uuid).await.ok().flatten();

            if let Some((current_version, updated_at, updated_by, _updated_by_name)) =
                current_metadata
            {
                if current_version == version_number {
                    // This is the current version, fetch from entity view using repository
                    if let Ok(Some(data_json)) =
                        repo.get_current_entity_data(uuid, &entity_type).await
                    {
                        let payload = VersionPayload {
                            version_number,
                            created_at: updated_at,
                            created_by: updated_by,
                            data: data_json,
                        };
                        return ApiResponse::ok(payload);
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Failed to get version: {}", e);
            return ApiResponse::<()>::internal_error("Failed to get version");
        }
    }

    ApiResponse::<()>::not_found("Version not found")
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
pub async fn query_entities(
    data: web::Data<ApiStateWrapper>,
    body: web::Json<EntityQueryRequest>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let repository = DynamicEntityRepository::new(data.db_pool().clone());

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
            let responses: Vec<DynamicEntityResponse> = entities
                .into_iter()
                .map(|e| DynamicEntityResponse {
                    entity_type: e.entity_type,
                    field_data: e.field_data,
                })
                .collect();

            ApiResponse::ok(responses)
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "status": "Error",
            "message": format!("Server error: {}", e),
        })),
    }
}
