#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use log::{debug, error, info};
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::admin::entity_definitions::conversions::entity_definition_to_schema_model;
use crate::admin::entity_definitions::models::EntityDefinitionSchema;
use crate::admin::entity_definitions::models::PaginationQuery;
use crate::admin::entity_definitions::models::{ApplySchemaRequest, PathUuid};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::response::ApiResponse;
use r_data_core_core::entity_definition::definition::EntityDefinition;

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
pub async fn list_entity_definitions(
    data: web::Data<ApiStateWrapper>,
    query: web::Query<PaginationQuery>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::check_permission_with_log(
        &auth.0,
        &ResourceNamespace::EntityDefinitions,
        &PermissionType::Read,
        None,
        "List entity definitions",
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to list entity definitions");
    }

    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    // Get both the entity definitions and the total count
    let (definitions_result, count_result) = tokio::join!(
        data.entity_definition_service()
            .list_entity_definitions(limit, offset),
        data.entity_definition_service().count_entity_definitions()
    );

    match (definitions_result, count_result) {
        (Ok(definitions), Ok(total)) => {
            // Convert to schema models
            let schema_definitions = definitions
                .iter()
                .map(entity_definition_to_schema_model)
                .collect::<Vec<_>>();

            ApiResponse::ok_paginated(schema_definitions, total, page, per_page)
        }
        (Err(e), _) => {
            error!("Failed to list entity definitions: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve entity definitions")
        }
        (_, Err(e)) => {
            error!("Failed to count entity definitions: {e}");
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
pub async fn get_entity_definition(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<PathUuid>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::check_permission_with_log(
        &auth.0,
        &ResourceNamespace::EntityDefinitions,
        &PermissionType::Read,
        None,
        "Get entity definition",
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to get entity definition");
    }
    match data
        .entity_definition_service()
        .get_entity_definition(&path.uuid)
        .await
    {
        Ok(definition) => {
            // Convert to schema model
            let schema_definition = entity_definition_to_schema_model(&definition);
            ApiResponse::ok(schema_definition)
        }
        Err(r_data_core_core::error::Error::NotFound(_)) => {
            ApiResponse::<()>::not_found("Entity definition")
        }
        Err(e) => {
            error!("Failed to retrieve entity definition: {e}");
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
pub async fn create_entity_definition(
    data: web::Data<ApiStateWrapper>,
    definition: web::Json<EntityDefinition>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::check_permission_with_log(
        &auth.0,
        &ResourceNamespace::EntityDefinitions,
        &PermissionType::Create,
        None,
        "Create entity definition",
    ) {
        return ApiResponse::<()>::forbidden(
            "Insufficient permissions to create entity definition",
        );
    }

    // Get authentication info from the RequiredAuth extractor
    let creator_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => {
            debug!("Required auth claims: {auth:?}");
            debug!("Parsed UUID from auth token: {uuid}");
            uuid
        }
        Err(e) => {
            error!(
                "Failed to parse UUID from claims.0.sub: {}, error: {}",
                auth.0.sub, e
            );
            return HttpResponse::InternalServerError().json(json!({
                "error": "No authentication claims found"
            }));
        }
    };

    // Extract definition and prepare for validation
    let mut entity_def = definition.into_inner();

    debug!("Creating entity definition");
    debug!("Creator UUID (from token): {creator_uuid}");

    // Set server-controlled fields
    let now = OffsetDateTime::now_utc();
    entity_def.created_at = now;
    entity_def.updated_at = now;
    entity_def.created_by = creator_uuid;
    entity_def.updated_by = Some(creator_uuid);
    entity_def.version = 1;

    // Ensure schema is properly initialized with entity_type
    if !entity_def.schema.properties.contains_key("entity_type") {
        let mut properties = entity_def.schema.properties.clone();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(entity_def.entity_type.clone()),
        );
        entity_def.schema = r_data_core_core::entity_definition::schema::Schema::new(properties);
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
        return ApiResponse::<()>::unprocessable_entity(&format!("Validation failed: {e}"));
    }

    // Create the entity definition using the service
    match data
        .entity_definition_service()
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
        Err(r_data_core_core::error::Error::ClassAlreadyExists(msg)) => {
            error!("Entity type already exists: {msg}");
            ApiResponse::<()>::conflict(&msg)
        }
        Err(r_data_core_core::error::Error::Validation(msg)) => {
            error!("Validation error when creating entity definition: {msg}");
            ApiResponse::<()>::unprocessable_entity(&msg)
        }
        Err(e) => {
            // Log the full error details
            error!(
                "Failed to create entity definition for {}: {:?}",
                entity_def.entity_type, e
            );

            // Add more detailed logging for diagnosis
            debug!("Class definition details: {entity_def:#?}");
            debug!("Error details: {e:#?}");

            ApiResponse::<()>::internal_error(&format!("Failed to create entity definition: {e}"))
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
pub async fn update_entity_definition(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<PathUuid>,
    definition: web::Json<EntityDefinition>,
    auth: RequiredAuth,
) -> impl Responder {
    // Get authentication info from the RequiredAuth extractor
    let updater_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            log::error!(
                "Failed to parse UUID from claims.0.sub: {}, error: {}",
                auth.0.sub,
                e
            );
            return HttpResponse::InternalServerError().json(json!({
                "error": "No authentication claims found"
            }));
        }
    };

    // First, get the existing definition to preserve system fields
    let existing_def = match data
        .entity_definition_service()
        .get_entity_definition(&path.uuid)
        .await
    {
        Ok(def) => def,
        Err(r_data_core_core::error::Error::NotFound(_)) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Class definition not found"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to retrieve entity definition: {e}")
            }));
        }
    };

    // Take user input and keep system fields
    let mut updated_def = definition.into_inner();
    updated_def.uuid = path.uuid; // Ensure UUID matches the path
    updated_def.entity_type = existing_def.entity_type;
    updated_def.created_at = existing_def.created_at;
    updated_def.created_by = existing_def.created_by;
    updated_def.updated_at = OffsetDateTime::now_utc();
    updated_def.updated_by = Some(updater_uuid);

    // Validate the definition
    if let Err(e) = updated_def.validate() {
        return HttpResponse::UnprocessableEntity().json(json!({
            "error": format!("Validation failed: {e}"),
        }));
    }

    // Update the entity definition
    match data
        .entity_definition_service()
        .update_entity_definition(&path.uuid, &updated_def)
        .await
    {
        Ok(()) => ApiResponse::ok(json!({
            "message": "Class definition updated successfully",
            "uuid": path.uuid
        })),
        Err(r_data_core_core::error::Error::Validation(msg)) => {
            ApiResponse::<()>::bad_request(&msg)
        }
        Err(e) => {
            error!("Failed to update entity definition: {e}");
            ApiResponse::<()>::internal_error(&format!("Failed to update entity definition: {e}"))
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
pub async fn delete_entity_definition(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<PathUuid>,
    auth: RequiredAuth,
) -> impl Responder {
    let actor_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(u) => u,
        Err(e) => {
            error!("Failed to parse UUID from claims: {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication");
        }
    };
    match data
        .entity_definition_service()
        .delete_entity_definition(&path.uuid, actor_uuid)
        .await
    {
        Ok(()) => ApiResponse::ok(json!({
            "message": "Class definition deleted successfully"
        })),
        Err(r_data_core_core::error::Error::NotFound(_)) => {
            ApiResponse::<()>::not_found("Entity definition")
        }
        Err(r_data_core_core::error::Error::Validation(msg)) => {
            ApiResponse::<()>::bad_request(&msg)
        }
        Err(e) => {
            error!("Failed to delete entity definition: {e}");
            ApiResponse::<()>::internal_error(&format!("Failed to delete entity definition: {e}"))
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
pub async fn apply_entity_definition_schema(
    data: web::Data<ApiStateWrapper>,
    body: web::Json<ApplySchemaRequest>,
    _: RequiredAuth,
) -> impl Responder {
    let uuid_option = body.uuid.as_ref();

    match data
        .entity_definition_service()
        .apply_schema(uuid_option)
        .await
    {
        Ok((_success_count, failed)) => {
            uuid_option.map_or_else(
                || {
                    // No UUID provided - return general success message
                    if failed.is_empty() {
                        ApiResponse::ok(json!({
                            "message": "Database schema applied successfully for all definitions"
                        }))
                    } else {
                        let (entity_type, _uuid, error) = &failed[0];
                        ApiResponse::<()>::internal_error(&format!(
                            "Failed to apply schema for {entity_type}: {error}"
                        ))
                    }
                },
                |uuid| {
                    // If a specific UUID was provided
                    if failed.is_empty() {
                        ApiResponse::ok(json!({
                            "message": "Database schema applied successfully",
                            "uuid": uuid
                        }))
                    } else {
                        let (entity_type, _uuid, error) = &failed[0];
                        ApiResponse::<()>::internal_error(&format!(
                            "Failed to apply schema for {entity_type}: {error}"
                        ))
                    }
                },
            )
        }
        Err(r_data_core_core::error::Error::NotFound(_)) => {
            ApiResponse::<()>::not_found("Entity definition")
        }
        Err(e) => {
            error!("Failed to apply schema: {e}");
            ApiResponse::<()>::internal_error(&format!("Failed to apply schema: {e}"))
        }
    }
}
