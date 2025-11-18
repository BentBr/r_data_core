use crate::api::auth::auth_enum;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use log::{debug, error, info};
use serde::Serialize;
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use super::models::ApplySchemaRequest;
use super::models::PathUuid;
use super::versioning_repository::EntityDefinitionVersioningRepository;
use crate::api::query::PaginationQuery;
use crate::api::response::ApiResponse;
use crate::api::ApiState;
use crate::entity::EntityDefinition;
use utoipa::ToSchema;

/// List entity definitions with pagination
#[utoipa::path(
    get,
    path = "/admin/api/v1/entity-definitions",
    tag = "entity-definitions",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Number of items per page (default: 20, max: 100)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return (alternative to per_page)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (alternative to page-based pagination)")
    ),
    responses(
        (status = 200, description = "List of entity definitions with pagination", body = Vec<EntityDefinitionSchema>),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("")]
async fn list_entity_definitions(
    data: web::Data<ApiState>,
    query: web::Query<PaginationQuery>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    // Get both the entity definitions and the total count
    let (definitions_result, count_result) = tokio::join!(
        data.entity_definition_service
            .list_entity_definitions(limit, offset),
        data.entity_definition_service.count_entity_definitions()
    );

    match (definitions_result, count_result) {
        (Ok(definitions), Ok(total)) => {
            // Convert to schema models
            let schema_definitions = definitions
                .iter()
                .map(|def| def.to_schema_model())
                .collect::<Vec<_>>();

            ApiResponse::ok_paginated(schema_definitions, total, page, per_page)
        }
        (Err(e), _) => {
            error!("Failed to list entity definitions: {}", e);
            ApiResponse::<()>::internal_error("Failed to retrieve entity definitions")
        }
        (_, Err(e)) => {
            error!("Failed to count entity definitions: {}", e);
            ApiResponse::<()>::internal_error("Failed to count entity definitions")
        }
    }
}

/// Get an entity definition by UUID
#[utoipa::path(
    get,
    path = "/admin/api/v1/entity-definitions/{uuid}",
    tag = "entity-definitions",
    params(
        ("uuid" = Uuid, Path, description = "Entity definition UUID")
    ),
    responses(
        (status = 200, description = "Entity definition found", body = EntityDefinitionSchema),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Entity definition not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/{uuid}")]
async fn get_entity_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    match data
        .entity_definition_service
        .get_entity_definition(&path.uuid)
        .await
    {
        Ok(definition) => {
            // Convert to schema model
            let schema_definition = definition.to_schema_model();
            ApiResponse::ok(schema_definition)
        }
        Err(crate::error::Error::NotFound(_)) => ApiResponse::<()>::not_found("Entity definition"),
        Err(e) => {
            error!("Failed to retrieve entity definition: {}", e);
            ApiResponse::<()>::internal_error("Failed to retrieve entity definition")
        }
    }
}

/// Create a new entity definition
#[utoipa::path(
    post,
    path = "/admin/api/v1/entity-definitions",
    tag = "entity-definitions",
    request_body = EntityDefinitionSchema,
    responses(
        (status = 201, description = "Class definition created successfully"),
        (status = 400, description = "Invalid input data"),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Validation failed"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("")]
async fn create_entity_definition(
    data: web::Data<ApiState>,
    definition: web::Json<EntityDefinition>,
    auth: auth_enum::RequiredAuth,
) -> impl Responder {
    // Get authentication info from the RequiredAuth extractor
    let creator_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => {
            debug!("Required auth claims: {:?}", auth);
            debug!("Parsed UUID from auth token: {}", uuid);
            uuid
        }
        Err(e) => {
            error!(
                "Failed to parse UUID from claims.0.sub: {}, error: {}",
                &auth.0.sub, e
            );
            return HttpResponse::InternalServerError().json(json!({
                "error": "No authentication claims found"
            }));
        }
    };

    // Extract definition and prepare for validation
    let mut entity_def = definition.into_inner();

    // Generate a new UUID if one wasn't provided
    if entity_def.uuid == Uuid::nil() {
        entity_def.uuid = Uuid::now_v7();
    }

    // Log UUIDs for debugging
    debug!("Class Definition UUID: {}", entity_def.uuid);
    debug!("Creator UUID (from token): {}", creator_uuid);

    // Set server-controlled fields
    let now = OffsetDateTime::now_utc();
    entity_def.created_at = now;
    entity_def.updated_at = now;
    entity_def.created_by = creator_uuid;
    entity_def.updated_by = Some(creator_uuid);
    entity_def.version = 1;

    // Ensure schema is properly initialized with entity_type
    if entity_def.schema.properties.get("entity_type").is_none() {
        let mut properties = entity_def.schema.properties.clone();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(entity_def.entity_type.clone()),
        );
        entity_def.schema = crate::entity::entity_definition::schema::Schema::new(properties);
        debug!(
            "Schema initialized with entity_type: {}",
            entity_def.entity_type
        );
    }

    // Log again after setting
    debug!(
        "Created_by after setting: {} (type: {})",
        entity_def.created_by,
        std::any::type_name_of_val(&entity_def.created_by)
    );

    // Validate entity definition
    if let Err(e) = entity_def.validate() {
        return ApiResponse::<()>::unprocessable_entity(&format!("Validation failed: {}", e));
    }

    // Create the entity definition using the service
    match data
        .entity_definition_service
        .create_entity_definition(&entity_def)
        .await
    {
        Ok(uuid) => {
            // Class definition created successfully
            info!(
                "Created entity definition for entity type: {}",
                entity_def.entity_type
            );

            ApiResponse::<serde_json::Value>::created(json!({
                "uuid": uuid,
                "message": "Class definition created successfully"
            }))
        }
        Err(crate::error::Error::ClassAlreadyExists(msg)) => {
            error!("Entity type already exists: {}", msg);
            ApiResponse::<()>::conflict(&msg)
        }
        Err(crate::error::Error::Validation(msg)) => {
            error!("Validation error when creating entity definition: {}", msg);
            ApiResponse::<()>::unprocessable_entity(&msg)
        }
        Err(e) => {
            // Log the full error details
            error!(
                "Failed to create entity definition for {}: {:?}",
                entity_def.entity_type, e
            );

            // Add more detailed logging for diagnosis
            debug!("Class definition details: {:#?}", entity_def);
            debug!("Error details: {:#?}", e);

            ApiResponse::<()>::internal_error(&format!("Failed to create entity definition: {}", e))
        }
    }
}

/// Update a entity definition
#[utoipa::path(
    put,
    path = "/admin/api/v1/entity-definitions/{uuid}",
    tag = "entity-definitions",
    params(
        ("uuid" = Uuid, Path, description = "Class definition UUID")
    ),
    request_body = EntityDefinitionSchema,
    responses(
        (status = 200, description = "Class definition updated successfully"),
        (status = 400, description = "Invalid input data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Class definition not found"),
        (status = 422, description = "Validation failed"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[put("/{uuid}")]
async fn update_entity_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    definition: web::Json<EntityDefinition>,
    auth: auth_enum::RequiredAuth,
) -> impl Responder {
    // Get authentication info from the RequiredAuth extractor
    let updater_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            log::error!(
                "Failed to parse UUID from claims.0.sub: {}, error: {}",
                &auth.0.sub,
                e
            );
            return HttpResponse::InternalServerError().json(json!({
                "error": "No authentication claims found"
            }));
        }
    };

    // First, get the existing definition to preserve system fields
    let existing_def = match data
        .entity_definition_service
        .get_entity_definition(&path.uuid)
        .await
    {
        Ok(def) => def,
        Err(crate::error::Error::NotFound(_)) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Class definition not found"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to retrieve entity definition: {}", e)
            }));
        }
    };

    // Take user input and keep system fields
    let mut updated_def = definition.into_inner();
    updated_def.uuid = path.uuid; // Ensure UUID matches the path
    updated_def.entity_type = existing_def.entity_type;
    updated_def.display_name = existing_def.display_name;
    updated_def.created_at = existing_def.created_at;
    updated_def.created_by = existing_def.created_by;
    updated_def.version = existing_def.version + 1; // Increment version
    updated_def.updated_at = OffsetDateTime::now_utc();
    updated_def.updated_by = Some(updater_uuid);

    // Validate the definition
    if let Err(e) = updated_def.validate() {
        return HttpResponse::UnprocessableEntity().json(json!({
            "error": format!("Validation failed: {}", e),
        }));
    }

    // Update the entity definition
    match data
        .entity_definition_service
        .update_entity_definition(&path.uuid, &updated_def)
        .await
    {
        Ok(_) => ApiResponse::ok(json!({
            "message": "Class definition updated successfully",
            "uuid": path.uuid
        })),
        Err(crate::error::Error::Validation(msg)) => ApiResponse::<()>::bad_request(&msg),
        Err(e) => {
            error!("Failed to update entity definition: {}", e);
            ApiResponse::<()>::internal_error(&format!("Failed to update entity definition: {}", e))
        }
    }
}

/// Delete a entity definition
#[utoipa::path(
    delete,
    path = "/admin/api/v1/entity-definitions/{uuid}",
    tag = "entity-definitions",
    params(
        ("uuid" = Uuid, Path, description = "Class definition UUID")
    ),
    responses(
        (status = 200, description = "Class definition deleted successfully"),
        (status = 400, description = "Cannot delete entity definition with existing entities"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Class definition not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{uuid}")]
async fn delete_entity_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    match data
        .entity_definition_service
        .delete_entity_definition(&path.uuid)
        .await
    {
        Ok(_) => ApiResponse::ok(json!({
            "message": "Class definition deleted successfully"
        })),
        Err(crate::error::Error::NotFound(_)) => ApiResponse::<()>::not_found("Entity definition"),
        Err(crate::error::Error::Validation(msg)) => ApiResponse::<()>::bad_request(&msg),
        Err(e) => {
            error!("Failed to delete entity definition: {}", e);
            ApiResponse::<()>::internal_error(&format!("Failed to delete entity definition: {}", e))
        }
    }
}

/// Apply database schema for entity definitions
#[utoipa::path(
    post,
    path = "/admin/api/v1/entity-definitions/apply-schema",
    tag = "entity-definitions",
    request_body(content = ApplySchemaRequest, description = "Optional entity definition UUID. If not provided, applies schema for all entity definitions"),
    responses(
        (status = 200, description = "Database schema applied successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Class definition not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/apply-schema")]
async fn apply_entity_definition_schema(
    data: web::Data<ApiState>,
    body: web::Json<ApplySchemaRequest>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let uuid_option = body.uuid.as_ref();

    match data
        .entity_definition_service
        .apply_schema(uuid_option)
        .await
    {
        Ok((success_count, failed)) => {
            if uuid_option.is_some() {
                // If a specific UUID was provided
                if failed.is_empty() {
                    ApiResponse::ok(json!({
                        "message": "Database schema applied successfully",
                        "uuid": uuid_option.unwrap()
                    }))
                } else {
                    let (entity_type, _uuid, error) = &failed[0];
                    ApiResponse::<()>::internal_error(&format!(
                        "Failed to apply schema for {}: {}",
                        entity_type, error
                    ))
                }
            } else {
                // If applying schema for all definitions
                if failed.is_empty() {
                    ApiResponse::ok(json!({
                        "message": format!("Applied schema for {} entity definitions", success_count)
                    }))
                } else {
                    ApiResponse::ok(json!({
                        "message": format!("Applied schema for {} entity definitions, {} failed", success_count, failed.len()),
                        "successful": success_count,
                        "failed": failed
                    }))
                }
            }
        }
        Err(crate::error::Error::NotFound(_)) => ApiResponse::<()>::not_found("Entity definition"),
        Err(e) => {
            error!("Failed to apply schema: {}", e);
            ApiResponse::<()>::internal_error(&format!("Failed to apply schema: {}", e))
        }
    }
}

/// Register routes for entity definitions
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_entity_definitions)
        .service(get_entity_definition)
        .service(create_entity_definition)
        .service(update_entity_definition)
        .service(delete_entity_definition)
        .service(apply_entity_definition_schema)
        .service(list_entity_fields_by_type)
        .service(list_entity_definition_versions)
        .service(get_entity_definition_version);
}

#[derive(Debug, Serialize, ToSchema)]
struct EntityFieldInfo {
    name: String,
    r#type: String,
    required: bool,
    system: bool,
}

/// List all fields for an entity definition by entity_type, including system fields
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
async fn list_entity_fields_by_type(
    data: web::Data<ApiState>,
    path: web::Path<String>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let entity_type = path.into_inner();
    match data
        .entity_definition_service
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
        Err(crate::error::Error::NotFound(_)) => ApiResponse::<()>::not_found("Entity definition"),
        Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to load fields: {}", e)),
    }
}

#[derive(serde::Serialize, ToSchema)]
pub struct EntityDefinitionVersionMeta {
    version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    created_at: time::OffsetDateTime,
    created_by: Option<Uuid>,
    created_by_name: Option<String>,
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
    data: web::Data<ApiState>,
    path: web::Path<Uuid>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let definition_uuid = path.into_inner();
    let versioning_repo = EntityDefinitionVersioningRepository::new(data.db_pool.clone());

    // Get historical versions
    let rows = match versioning_repo.list_definition_versions(definition_uuid).await {
        Ok(rows) => rows,
        Err(e) => {
            error!("Failed to list entity definition versions: {}", e);
            return ApiResponse::<()>::internal_error("Failed to list versions");
        }
    };

    // Get current definition metadata
    let current_metadata = match versioning_repo.get_current_definition_metadata(definition_uuid).await {
        Ok(metadata) => metadata,
        Err(e) => {
            error!("Failed to get current entity definition metadata: {}", e);
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
    out.sort_by(|a, b| b.version_number.cmp(&a.version_number));

    ApiResponse::ok(out)
}

#[derive(serde::Serialize, ToSchema)]
pub struct EntityDefinitionVersionPayload {
    version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    created_at: time::OffsetDateTime,
    created_by: Option<Uuid>,
    data: serde_json::Value,
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
    data: web::Data<ApiState>,
    path: web::Path<(Uuid, i32)>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let (definition_uuid, version_number) = path.into_inner();
    let versioning_repo = EntityDefinitionVersioningRepository::new(data.db_pool.clone());

    // First try to get from versions table
    match versioning_repo.get_definition_version(definition_uuid, version_number).await {
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
            let current_metadata = versioning_repo.get_current_definition_metadata(definition_uuid).await.ok().flatten();

            if let Some((current_version, updated_at, updated_by, _updated_by_name)) = current_metadata {
                if current_version == version_number {
                    // This is the current version, fetch from entity_definitions table
                    match data.entity_definition_service.get_entity_definition(&definition_uuid).await {
                        Ok(def) => {
                            let current_json = serde_json::to_value(&def).unwrap_or(serde_json::json!({}));
                            let payload = EntityDefinitionVersionPayload {
                                version_number,
                                created_at: updated_at,
                                created_by: updated_by,
                                data: current_json,
                            };
                            return ApiResponse::ok(payload);
                        }
                        Err(crate::error::Error::NotFound(_)) => {}
                        Err(e) => {
                            error!("Failed to get entity definition: {}", e);
                            return ApiResponse::<()>::internal_error("Failed to get entity definition");
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to get entity definition version: {}", e);
            return ApiResponse::<()>::internal_error("Failed to get version");
        }
    }

    ApiResponse::<()>::not_found("Version not found")
}
