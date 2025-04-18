use actix_web::{delete, get, post, put, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use log::info;
use serde_json::json;
use uuid::Uuid;
use crate::api::auth::auth_enum;

use super::models::ApplySchemaRequest;
use super::models::PaginationQuery;
use super::models::PathUuid;
use super::repository::ClassDefinitionRepository;
use crate::api::jwt::AuthUserClaims;
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
        Ok(definition) => {
            // Convert to schema model
            let schema_definition = definition.to_schema_model();
            HttpResponse::Ok().json(schema_definition)
        }
        Err(_) => HttpResponse::NotFound().json(json!({
            "error": "Class definition not found"
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
    req: HttpRequest,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    // Get authenticated user from JWT claims
    let creator_uuid = match req.extensions().get::<AuthUserClaims>() {
        Some(claims) => match Uuid::parse_str(&claims.sub) {
            Ok(uuid) => Some(uuid),
            Err(_) => None,
        },
        None => None,
    };

    // Extract definition and prepare for validation
    let mut class_def = definition.into_inner();

    // Generate a new UUID if one wasn't provided
    if class_def.uuid == Uuid::nil() {
        let context = uuid::ContextV7::new();
        let ts = uuid::timestamp::Timestamp::now(&context);
        class_def.uuid = Uuid::new_v7(ts);
    }

    // Set server-controlled fields
    let now = chrono::Utc::now();
    class_def.created_at = now;
    class_def.updated_at = now;
    class_def.created_by = creator_uuid;
    class_def.updated_by = creator_uuid;
    class_def.version = 1;

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
            // Create the database table for this entity type
            info!(
                "Creating database table for entity type: {}",
                class_def.entity_type
            );

            // Update the entity table structure
            match repository
                .update_entity_table_for_class_definition(&class_def)
                .await
            {
                Ok(_) => HttpResponse::Created().json(json!({
                    "uuid": uuid,
                    "message": "Class definition created successfully with database table"
                })),
                Err(e) => {
                    // Table creation failed, but definition was saved
                    HttpResponse::Ok().json(json!({
                        "uuid": uuid,
                        "warning": format!("Class definition created but table creation failed: {}", e),
                        "message": "Class definition was saved but will not be usable until the table is created"
                    }))
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to create class definition: {}", e)
        })),
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
    req: HttpRequest,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    // Get authenticated user from JWT claims
    let updater_uuid = match req.extensions().get::<AuthUserClaims>() {
        Some(claims) => match Uuid::parse_str(&claims.sub) {
            Ok(uuid) => Some(uuid),
            Err(_) => None,
        },
        None => None,
    };

    // Get existing definition
    let existing_def = match repository.get_by_uuid(&path.uuid).await {
        Ok(def) => def,
        Err(_) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Class definition not found"
            }));
        }
    };

    // Extract and prepare the updated definition
    let mut updated_def = definition.into_inner();

    // Ensure UUID remains the same
    if updated_def.uuid != path.uuid {
        return HttpResponse::BadRequest().json(json!({
            "error": "UUID in path does not match UUID in body"
        }));
    }

    // Preserve immutable fields
    updated_def.created_at = existing_def.created_at;
    updated_def.created_by = existing_def.created_by;
    updated_def.version = existing_def.version + 1; // Increment version

    // Update mutable fields
    updated_def.updated_at = chrono::Utc::now();
    updated_def.updated_by = updater_uuid;

    // Validate updated definition
    if let Err(e) = updated_def.validate() {
        return HttpResponse::UnprocessableEntity().json(json!({
            "error": format!("Validation failed: {}", e)
        }));
    }

    // Check if entity type changed and ensure it doesn't conflict with an existing one
    if updated_def.entity_type != existing_def.entity_type {
        match repository
            .get_by_entity_type(&updated_def.entity_type)
            .await
        {
            Ok(Some(_)) => {
                return HttpResponse::BadRequest().json(json!({
                    "error": format!("Cannot change entity type to '{}' as it already exists", updated_def.entity_type)
                }));
            }
            Ok(None) => {} // Entity type doesn't exist, proceed
            Err(e) => {
                return HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to check if entity type exists: {}", e)
                }));
            }
        }
    }

    // Update class definition
    match repository.update(&path.uuid, &updated_def).await {
        Ok(_) => {
            // Update the database table structure for this entity type
            info!(
                "Updating database table for entity type: {}",
                updated_def.entity_type
            );

            // Update the entity table structure
            match repository.update_entity_table_for_class_definition(&updated_def).await {
                Ok(_) => {
                    HttpResponse::Ok().json(json!({
                        "message": "Class definition and database table updated successfully"
                    }))
                },
                Err(e) => {
                    HttpResponse::Ok().json(json!({
                        "warning": format!("Class definition updated but table update failed: {}", e),
                        "message": "Class definition was saved but table structure may not match the definition"
                    }))
                }
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

    // First get the class definition to know its table name
    let definition = match repository.get_by_uuid(&path.uuid).await {
        Ok(def) => def,
        Err(_) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Class definition not found"
            }));
        }
    };

    // Check if there are any entities of this type
    let table_name = definition.get_table_name();
    let table_exists = match repository.check_table_exists(&table_name).await {
        Ok(exists) => exists,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to check if table exists: {}", e)
            }));
        }
    };

    if table_exists {
        // Check if there are any records in the table
        let count = match repository.count_table_records(&table_name).await {
            Ok(count) => count,
            Err(e) => {
                return HttpResponse::InternalServerError().json(json!({
                    "error": format!("Failed to count records in table: {}", e)
                }));
            }
        };

        if count > 0 {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Cannot delete class definition with existing entities. There are {} entities in table {}.", count, table_name)
            }));
        }
    }

    // Delete the class definition
    match repository.delete(&path.uuid).await {
        Ok(_) => {
            // Drop the table if it exists
            if table_exists {
                let drop_sql = format!("DROP TABLE IF EXISTS {} CASCADE", table_name);
                match repository.apply_schema(&drop_sql).await {
                    Ok(_) => {
                        // Also remove from the entity registry if it exists
                        let _ = repository.delete_from_entities_registry(&definition.entity_type).await;

                        HttpResponse::Ok().json(json!({
                            "message": "Class definition and associated table deleted successfully"
                        }))
                    },
                    Err(e) => {
                        HttpResponse::Ok().json(json!({
                            "warning": format!("Class definition deleted but failed to drop table: {}", e),
                            "message": "Class definition was deleted but the database table may still exist"
                        }))
                    }
                }
            } else {
                HttpResponse::Ok().json(json!({
                    "message": "Class definition deleted successfully"
                }))
            }
        }
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
    log::info!(
        "apply_class_definition_schema endpoint called with body: {:?}",
        body
    );

    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    // If UUID is provided, apply schema for specific class definition
    if let Some(uuid) = body.uuid {
        match repository.get_by_uuid(&uuid).await {
            Ok(definition) => {
                // Update table structure using the columnar approach
                match repository
                    .update_entity_table_for_class_definition(&definition)
                    .await
                {
                    Ok(_) => HttpResponse::Ok().json(json!({
                        "success": true,
                        "entity_type": definition.entity_type,
                        "uuid": definition.uuid.to_string(),
                        "message": "Database schema applied successfully"
                    })),
                    Err(e) => HttpResponse::InternalServerError().json(json!({
                        "success": false,
                        "entity_type": definition.entity_type,
                        "uuid": definition.uuid.to_string(),
                        "error": format!("Failed to apply database schema: {}", e)
                    })),
                }
            }
            Err(e) => HttpResponse::NotFound().json(json!({
                "success": false,
                "error": format!("Class definition not found: {}", e)
            })),
        }
    } else {
        // Apply schema for all class definitions
        match repository.list(1000, 0).await {
            Ok(definitions) => {
                let mut results = Vec::new();

                for definition in definitions {
                    // Update table structure using the columnar approach
                    let result = match repository
                        .update_entity_table_for_class_definition(&definition)
                        .await
                    {
                        Ok(_) => json!({
                            "success": true,
                            "entity_type": definition.entity_type,
                            "uuid": definition.uuid.to_string(),
                            "message": "Database schema applied successfully"
                        }),
                        Err(e) => json!({
                            "success": false,
                            "entity_type": definition.entity_type,
                            "uuid": definition.uuid.to_string(),
                            "error": format!("Failed to apply database schema: {}", e)
                        }),
                    };
                    results.push(result);
                }

                // Clean up any unused entity tables
                let cleanup_result = match repository.cleanup_unused_entity_tables().await {
                    Ok(_) => "Unused entity tables have been cleaned up".to_string(),
                    Err(e) => format!("Warning: Failed to clean up unused entity tables: {}", e),
                };

                HttpResponse::Ok().json(json!({
                    "results": results,
                    "total": results.len(),
                    "cleanup": cleanup_result,
                    "message": "Database schema application process completed"
                }))
            }
            Err(e) => HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to list class definitions: {}", e)
            })),
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
