use crate::api::auth::auth_enum;
use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder};
use log::{debug, error, info};
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use super::models::ApplySchemaRequest;
use super::models::PaginationQuery;
use super::models::PathUuid;
use super::repository::ClassDefinitionRepository;
use crate::api::jwt::AuthUserClaims;
use crate::api::ApiState;
use crate::entity::class::repository_trait::ClassDefinitionRepositoryTrait;
use crate::entity::ClassDefinition;

/// List class definitions
#[utoipa::path(
    get,
    path = "/admin/api/v1/class-definitions",
    tag = "class-definitions",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip")
    ),
    responses(
        (status = 200, description = "List of class definitions", body = Vec<ClassDefinitionSchema>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("")]
async fn list_class_definitions(
    data: web::Data<ApiState>,
    query: web::Query<PaginationQuery>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    match repository.list(limit, offset).await {
        Ok(definitions) => {
            // Convert to schema models
            let schema_definitions = definitions
                .iter()
                .map(|def| def.to_schema_model())
                .collect::<Vec<_>>();

            HttpResponse::Ok().json(schema_definitions)
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to list class definitions: {}", e)
        })),
    }
}

/// Get a class definition by UUID
#[utoipa::path(
    get,
    path = "/admin/api/v1/class-definitions/{uuid}",
    tag = "class-definitions",
    params(
        ("uuid" = Uuid, Path, description = "Class definition UUID")
    ),
    responses(
        (status = 200, description = "Class definition found", body = ClassDefinitionSchema),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Class definition not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/{uuid}")]
async fn get_class_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    match repository.get_by_uuid(&path.uuid).await {
        Ok(Some(definition)) => {
            // Convert to schema model
            let schema_definition = definition.to_schema_model();
            HttpResponse::Ok().json(schema_definition)
        }
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error": "Class definition not found"
        })),
        Err(_) => HttpResponse::InternalServerError().json(json!({
            "error": "Failed to retrieve class definition"
        })),
    }
}

/// Create a new class definition
#[utoipa::path(
    post,
    path = "/admin/api/v1/class-definitions",
    tag = "class-definitions",
    request_body = ClassDefinitionSchema,
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
async fn create_class_definition(
    data: web::Data<ApiState>,
    definition: web::Json<ClassDefinition>,
    auth: auth_enum::RequiredAuth,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

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
    let mut class_def = definition.into_inner();

    // Generate a new UUID if one wasn't provided
    if class_def.uuid == Uuid::nil() {
        class_def.uuid = Uuid::now_v7();
    }

    // Log UUIDs for debugging
    debug!("Class Definition UUID: {}", class_def.uuid);
    debug!("Creator UUID (from token): {}", creator_uuid);

    // Set server-controlled fields
    let now = OffsetDateTime::now_utc();
    class_def.created_at = now;
    class_def.updated_at = now;
    class_def.created_by = creator_uuid;
    class_def.updated_by = Some(creator_uuid);
    class_def.version = 1;

    // Log again after setting
    log::debug!(
        "Created_by after setting: {} (type: {})",
        class_def.created_by,
        std::any::type_name_of_val(&class_def.created_by)
    );

    // Validate class definition
    if let Err(e) = class_def.validate() {
        return HttpResponse::UnprocessableEntity().json(json!({
            "error": format!("Validation failed: {}", e),
        }));
    }

    // Check if entity type already exists
    match repository.get_by_entity_type(&class_def.entity_type).await {
        Ok(Some(_)) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Entity type '{}' already exists", class_def.entity_type),
            }));
        }
        Ok(None) => {} // Entity type doesn't exist, proceed
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to check if entity type exists: {}", e),
            }));
        }
    }

    // Save class definition
    match repository.create(&class_def).await {
        Ok(uuid) => {
            // Class definition created successfully
            info!(
                "Created class definition for entity type: {}",
                class_def.entity_type
            );

            HttpResponse::Created().json(json!({
                "uuid": uuid,
                "message": "Class definition created successfully"
            }))
        }
        Err(e) => {
            // Log the full error details
            error!(
                "Failed to create class definition for {}: {:?}",
                class_def.entity_type, e
            );
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to create class definition: {}", e),
                "details": format!("{:?}", e)
            }))
        }
    }
}

/// Update a class definition
#[utoipa::path(
    put,
    path = "/admin/api/v1/class-definitions/{uuid}",
    tag = "class-definitions",
    params(
        ("uuid" = Uuid, Path, description = "Class definition UUID")
    ),
    request_body = ClassDefinitionSchema,
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
async fn update_class_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    definition: web::Json<ClassDefinition>,
    auth: auth_enum::RequiredAuth,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

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

    // Check if the class definition exists
    let existing_def = match repository.get_by_uuid(&path.uuid).await {
        Ok(Some(def)) => def,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Class definition not found"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to retrieve class definition: {}", e)
            }));
        }
    };

    // Take user input and keep system fields
    let mut updated_def = definition.into_inner();
    updated_def.uuid = path.uuid; // Ensure UUID matches the path
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

    // Entity type cannot be changed if the entity type is already used
    if updated_def.entity_type != existing_def.entity_type {
        match repository
            .get_by_entity_type(&updated_def.entity_type)
            .await
        {
            Ok(Some(_)) => {
                return HttpResponse::BadRequest().json(json!({
                    "error": format!("Entity type '{}' already exists", updated_def.entity_type),
                }));
            }
            Ok(None) => {} // Entity type is available
            Err(e) => {
                return HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to check entity type: {}", e),
                }));
            }
        }
    }

    // Update the class definition
    match repository.update(&path.uuid, &updated_def).await {
        Ok(_) => {
            // Now update the database table structure
            match repository
                .update_entity_view_for_class_definition(&updated_def)
                .await
            {
                Ok(_) => HttpResponse::Ok().json(json!({
                    "message": "Class definition updated successfully",
                    "uuid": path.uuid
                })),
                Err(e) => HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to update table structure: {}", e)
                })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to update class definition: {}", e)
        })),
    }
}

/// Delete a class definition
#[utoipa::path(
    delete,
    path = "/admin/api/v1/class-definitions/{uuid}",
    tag = "class-definitions",
    params(
        ("uuid" = Uuid, Path, description = "Class definition UUID")
    ),
    responses(
        (status = 200, description = "Class definition deleted successfully"),
        (status = 400, description = "Cannot delete class definition with existing entities"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Class definition not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{uuid}")]
async fn delete_class_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    // Get the class definition
    let definition = match repository.get_by_uuid(&path.uuid).await {
        Ok(Some(def)) => def,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Class definition not found"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to retrieve class definition: {}", e)
            }));
        }
    };

    // Check if the table exists
    let table_name = definition.get_table_name();
    let table_exists = match repository.check_view_exists(&table_name).await {
        Ok(exists) => exists,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to check if table exists: {}", e)
            }));
        }
    };

    // If the table exists, check for records
    if table_exists {
        let count = match repository.count_view_records(&table_name).await {
            Ok(c) => c,
            Err(e) => {
                return HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to count records: {}", e)
                }));
            }
        };

        if count > 0 {
            return HttpResponse::UnprocessableEntity().json(json!({
                "error": format!("Cannot delete class definition that has {} entities. Delete all entities first.", count)
            }));
        }
    }

    // Delete the class definition
    match repository.delete(&path.uuid).await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "message": "Class definition deleted successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to delete class definition: {}", e)
        })),
    }
}

/// Apply database schema for class definitions
#[utoipa::path(
    post,
    path = "/admin/api/v1/class-definitions/apply-schema",
    tag = "class-definitions",
    request_body(content = ApplySchemaRequest, description = "Optional class definition UUID. If not provided, applies schema for all class definitions"),
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
async fn apply_class_definition_schema(
    data: web::Data<ApiState>,
    body: web::Json<ApplySchemaRequest>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    // Validate UUID if provided
    if let Some(uuid) = body.uuid {
        match repository.get_by_uuid(&uuid).await {
            Ok(Some(definition)) => {
                match repository
                    .update_entity_view_for_class_definition(&definition)
                    .await
                {
                    Ok(_) => {
                        return HttpResponse::Ok().json(json!({
                            "message": "Database schema applied successfully",
                            "uuid": definition.uuid,
                            "entity_type": definition.entity_type,
                        }));
                    }
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(json!({
                            "error": format!("Failed to apply schema: {}", e),
                            "uuid": definition.uuid,
                            "entity_type": definition.entity_type,
                        }));
                    }
                }
            }
            Ok(None) => {
                return HttpResponse::NotFound().json(json!({
                    "error": format!("Class definition with UUID {} not found", uuid)
                }));
            }
            Err(e) => {
                return HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to retrieve class definition: {}", e)
                }));
            }
        }
    }

    // If no UUID, apply schema for all class definitions
    match repository.list(1000, 0).await {
        Ok(definitions) => {
            let mut success_count = 0;
            let mut failed = Vec::new();

            for definition in definitions {
                match repository
                    .update_entity_view_for_class_definition(&definition)
                    .await
                {
                    Ok(_) => {
                        success_count += 1;
                    }
                    Err(e) => {
                        failed.push((
                            definition.entity_type.clone(),
                            definition.uuid,
                            e.to_string(),
                        ));
                    }
                }
            }

            if failed.is_empty() {
                HttpResponse::Ok().json(json!({
                    "message": format!("Applied schema for {} class definitions", success_count)
                }))
            } else {
                HttpResponse::PartialContent().json(json!({
                    "message": format!("Applied schema for {} class definitions, {} failed", success_count, failed.len()),
                    "successful": success_count,
                    "failed": failed
                }))
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to list class definitions: {}", e)
        })),
    }
}

/// Register routes for class definitions
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_class_definitions)
        .service(get_class_definition)
        .service(create_class_definition)
        .service(update_class_definition)
        .service(delete_class_definition)
        .service(apply_class_definition_schema);
}
