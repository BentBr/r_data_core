use crate::api::auth::auth_enum;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use log::{debug, error, info};
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use super::models::ApplySchemaRequest;
use super::models::PaginationQuery;
use super::models::PathUuid;
use crate::api::ApiState;
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
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    match data.class_definition_service.list_class_definitions(limit, offset).await {
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
    match data.class_definition_service.get_class_definition(&path.uuid).await {
        Ok(definition) => {
            // Convert to schema model
            let schema_definition = definition.to_schema_model();
            HttpResponse::Ok().json(schema_definition)
        }
        Err(crate::error::Error::NotFound(_)) => HttpResponse::NotFound().json(json!({
            "error": "Class definition not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to retrieve class definition: {}", e)
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
    
    // Ensure schema is properly initialized with entity_type
    if class_def.schema.properties.get("entity_type").is_none() {
        let mut properties = class_def.schema.properties.clone();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(class_def.entity_type.clone()),
        );
        class_def.schema = crate::entity::class::schema::Schema::new(properties);
        debug!("Schema initialized with entity_type: {}", class_def.entity_type);
    }

    // Log again after setting
    debug!(
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

    // Create the class definition using the service
    match data.class_definition_service.create_class_definition(&class_def).await {
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
        Err(crate::error::Error::Validation(msg)) => {
            error!("Validation error when creating class definition: {}", msg);
            HttpResponse::UnprocessableEntity().json(json!({
                "error": msg
            }))
        }
        Err(e) => {
            // Log the full error details
            error!(
                "Failed to create class definition for {}: {:?}",
                class_def.entity_type, e
            );
            
            // Add more detailed logging for diagnosis
            debug!("Class definition details: {:#?}", class_def);
            debug!("Error details: {:#?}", e);
            
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
    let existing_def = match data.class_definition_service.get_class_definition(&path.uuid).await {
        Ok(def) => def,
        Err(crate::error::Error::NotFound(_)) => {
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

    // Update the class definition
    match data.class_definition_service.update_class_definition(&path.uuid, &updated_def).await {
        Ok(_) => {
            HttpResponse::Ok().json(json!({
                "message": "Class definition updated successfully",
                "uuid": path.uuid
            }))
        }
        Err(crate::error::Error::Validation(msg)) => {
            HttpResponse::BadRequest().json(json!({
                "error": msg
            }))
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
    match data.class_definition_service.delete_class_definition(&path.uuid).await {
        Ok(_) => HttpResponse::Ok().json(json!({
            "message": "Class definition deleted successfully"
        })),
        Err(crate::error::Error::NotFound(_)) => HttpResponse::NotFound().json(json!({
            "error": "Class definition not found"
        })),
        Err(crate::error::Error::Validation(msg)) => HttpResponse::BadRequest().json(json!({
            "error": msg
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
    let uuid_option = body.uuid.as_ref();
    
    match data.class_definition_service.apply_schema(uuid_option).await {
        Ok((success_count, failed)) => {
            if uuid_option.is_some() {
                // If a specific UUID was provided
                if failed.is_empty() {
                    HttpResponse::Ok().json(json!({
                        "message": "Database schema applied successfully",
                        "uuid": uuid_option.unwrap()
                    }))
                } else {
                    let (entity_type, uuid, error) = &failed[0];
                    HttpResponse::InternalServerError().json(json!({
                        "error": format!("Failed to apply schema for {}: {}", entity_type, error),
                        "uuid": uuid
                    }))
                }
            } else {
                // If applying schema for all definitions
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
        },
        Err(crate::error::Error::NotFound(_)) => {
            HttpResponse::NotFound().json(json!({
                "error": format!("Class definition with UUID {} not found", uuid_option.unwrap())
            }))
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to apply schema: {}", e)
            }))
        }
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
