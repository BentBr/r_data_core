#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, web, Responder};
use log::error;
use serde::Serialize;
use uuid::Uuid;

use crate::admin::entity_definitions::models::{
    EntityDefinitionVersionMeta, EntityDefinitionVersionPayload,
};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::response::ApiResponse;
use r_data_core_persistence::EntityDefinitionVersioningRepository;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct EntityFieldInfo {
    pub name: String,
    pub r#type: String,
    pub required: bool,
    pub system: bool,
}

/// List all fields for an entity definition by `entity_type`, including system fields
#[utoipa::path(
    get,
    path = "/admin/api/v1/entity-definitions/{entity_type}/fields",
    tag = "entity-definitions",
    params(("entity_type" = String, Path, description = "Entity type identifier")),
    responses(
        (status = 200, description = "Fields for entity type (including system fields)", body = [EntityFieldInfo]),
        (status = 404, description = "Entity definition not found")
    ),
    security(("jwt" = []))
)]
#[get("/{entity_type}/fields")]
pub async fn list_entity_fields_by_type(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<String>,
    _: RequiredAuth,
) -> impl Responder {
    let entity_type = path.into_inner();
    match data
        .entity_definition_service()
        .list_fields_with_system_by_entity_type(&entity_type)
        .await
    {
        Ok(items) => {
            let api_items: Vec<EntityFieldInfo> = items
                .into_iter()
                .map(|i| EntityFieldInfo {
                    name: i.name,
                    r#type: i.field_type,
                    required: i.required,
                    system: i.system,
                })
                .collect::<Vec<EntityFieldInfo>>();
            ApiResponse::ok(api_items)
        }
        Err(r_data_core_core::error::Error::NotFound(_)) => {
            ApiResponse::<()>::not_found("Entity definition")
        }
        Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to load fields: {e}")),
    }
}

/// List versions of an entity definition
#[utoipa::path(
    get,
    path = "/admin/api/v1/entity-definitions/{uuid}/versions",
    tag = "entity-definitions",
    params(
        ("uuid" = Uuid, Path, description = "Entity definition UUID")
    ),
    responses(
        (status = 200, description = "List of versions", body = Vec<EntityDefinitionVersionMeta>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Entity definition not found"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/{uuid}/versions")]
pub async fn list_entity_definition_versions(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    _: RequiredAuth,
) -> impl Responder {
    let definition_uuid = path.into_inner();
    let versioning_repo = EntityDefinitionVersioningRepository::new(data.db_pool().clone());

    // Get historical versions
    let rows = match versioning_repo
        .list_definition_versions(definition_uuid)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            error!("Failed to list entity definition versions: {e}");
            return ApiResponse::<()>::internal_error("Failed to list versions");
        }
    };

    // Get current definition metadata
    let current_metadata = match versioning_repo
        .get_current_definition_metadata(definition_uuid)
        .await
    {
        Ok(metadata) => metadata,
        Err(e) => {
            error!("Failed to get current entity definition metadata: {e}");
            return ApiResponse::<()>::internal_error("Failed to get current metadata");
        }
    };

    let mut out: Vec<EntityDefinitionVersionMeta> = Vec::new();

    // Add current version if it exists and is not in the versions table
    if let Some((version, updated_at, updated_by, updated_by_name)) = current_metadata {
        let is_in_versions = rows.iter().any(|r| r.version_number == version);
        if !is_in_versions {
            out.push(EntityDefinitionVersionMeta {
                version_number: version,
                created_at: updated_at,
                created_by: updated_by,
                created_by_name: updated_by_name,
            });
        }
    }

    // Add all historical versions
    for r in rows {
        out.push(EntityDefinitionVersionMeta {
            version_number: r.version_number,
            created_at: r.created_at,
            created_by: r.created_by,
            created_by_name: r.created_by_name,
        });
    }

    // Sort by version number descending (newest first)
    out.sort_by_key(|b| std::cmp::Reverse(b.version_number));

    ApiResponse::ok(out)
}

/// Get a specific version snapshot of an entity definition
#[utoipa::path(
    get,
    path = "/admin/api/v1/entity-definitions/{uuid}/versions/{version_number}",
    tag = "entity-definitions",
    params(
        ("uuid" = Uuid, Path, description = "Entity definition UUID"),
        ("version_number" = i32, Path, description = "Version number")
    ),
    responses(
        (status = 200, description = "Version snapshot", body = EntityDefinitionVersionPayload),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Version not found"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/{uuid}/versions/{version_number}")]
pub async fn get_entity_definition_version(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<(Uuid, i32)>,
    _: RequiredAuth,
) -> impl Responder {
    let (definition_uuid, version_number) = path.into_inner();
    let versioning_repo = EntityDefinitionVersioningRepository::new(data.db_pool().clone());

    // First try to get from versions table
    match versioning_repo
        .get_definition_version(definition_uuid, version_number)
        .await
    {
        Ok(Some(row)) => {
            let payload = EntityDefinitionVersionPayload {
                version_number: row.version_number,
                created_at: row.created_at,
                created_by: row.created_by,
                data: row.data,
            };
            return ApiResponse::ok(payload);
        }
        Ok(None) => {
            // Not in versions table, check if it's the current version
            let current_metadata = versioning_repo
                .get_current_definition_metadata(definition_uuid)
                .await
                .ok()
                .flatten();

            if let Some((current_version, updated_at, updated_by, _updated_by_name)) =
                current_metadata
            {
                if current_version == version_number {
                    // This is the current version, fetch from entity_definitions table
                    match data
                        .entity_definition_service()
                        .get_entity_definition(&definition_uuid)
                        .await
                    {
                        Ok(def) => {
                            let current_json = serde_json::to_value(&def)
                                .unwrap_or_else(|_| serde_json::json!({}));
                            let payload = EntityDefinitionVersionPayload {
                                version_number,
                                created_at: updated_at,
                                created_by: updated_by,
                                data: current_json,
                            };
                            return ApiResponse::ok(payload);
                        }
                        Err(r_data_core_core::error::Error::NotFound(_)) => {}
                        Err(e) => {
                            error!("Failed to get entity definition: {e}");
                            return ApiResponse::<()>::internal_error(
                                "Failed to get entity definition",
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to get entity definition version: {e}");
            return ApiResponse::<()>::internal_error("Failed to get version");
        }
    }

    ApiResponse::<()>::not_found("Version not found")
}
