use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use log::info;
use serde_json::json;

use super::models::PaginationQuery;
use super::models::PathUuid;
use super::repository::ClassDefinitionRepository;
use crate::api::ApiState;
use crate::entity::ClassDefinition;

/// List class definitions
#[utoipa::path(
    get,
    path = "/admin/api/v1/class-definitions",
    tag = "admin",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip")
    ),
    responses(
        (status = 200, description = "List of class definitions", body = Vec<ClassDefinition>),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt_auth" = [])
    )
)]
#[get("/class-definitions")]
async fn list_class_definitions(
    data: web::Data<ApiState>,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    match repository.list(limit, offset).await {
        Ok(definitions) => HttpResponse::Ok().json(definitions),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to list class definitions: {}", e)
        })),
    }
}

/// Get a class definition by UUID
#[utoipa::path(
    get,
    path = "/admin/api/v1/class-definitions/{uuid}",
    tag = "admin",
    params(
        ("uuid" = Uuid, Path, description = "Class definition UUID")
    ),
    responses(
        (status = 200, description = "Class definition found", body = ClassDefinition),
        (status = 404, description = "Class definition not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt_auth" = [])
    )
)]
#[get("/class-definitions/{uuid}")]
async fn get_class_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    match repository.get_by_uuid(&path.uuid).await {
        Ok(definition) => HttpResponse::Ok().json(definition),
        Err(_) => HttpResponse::NotFound().json(json!({
            "error": "Class definition not found"
        })),
    }
}

/// Create a new class definition
#[utoipa::path(
    post,
    path = "/admin/api/v1/class-definitions",
    tag = "admin",
    request_body = ClassDefinition,
    responses(
        (status = 201, description = "Class definition created successfully"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt_auth" = [])
    )
)]
#[post("/class-definitions")]
async fn create_class_definition(
    data: web::Data<ApiState>,
    definition: web::Json<ClassDefinition>,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    // Save class definition
    match repository.create(&definition).await {
        Ok(uuid) => {
            // Get the created definition with UUID
            let created_def = definition.into_inner();

            // Create the database table for this entity type
            info!(
                "Creating database table for entity type: {}",
                created_def.entity_type
            );

            // Use the specific apply_to_database method
            let schema_sql = match created_def.schema.generate_sql_schema() {
                Ok(sql) => sql,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(json!({
                        "error": format!("Failed to generate SQL schema: {}", e)
                    }));
                }
            };

            match repository.apply_schema(&schema_sql).await {
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
    tag = "admin",
    params(
        ("uuid" = Uuid, Path, description = "Class definition UUID")
    ),
    request_body = ClassDefinition,
    responses(
        (status = 200, description = "Class definition updated successfully"),
        (status = 404, description = "Class definition not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt_auth" = [])
    )
)]
#[put("/class-definitions/{uuid}")]
async fn update_class_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    definition: web::Json<ClassDefinition>,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let repository = ClassDefinitionRepository::new(db_pool.clone());

    // Update class definition
    match repository.update(&path.uuid, &definition).await {
        Ok(_) => {
            // Update the database table structure for this entity type
            let updated_def = definition.into_inner();

            info!(
                "Updating database table for entity type: {}",
                updated_def.entity_type
            );

            // Use the specific apply_to_database method
            let schema_sql = match updated_def.schema.generate_sql_schema() {
                Ok(sql) => sql,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(json!({
                        "error": format!("Failed to generate SQL schema: {}", e)
                    }));
                }
            };

            match repository.apply_schema(&schema_sql).await {
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
    tag = "admin",
    params(
        ("uuid" = Uuid, Path, description = "Class definition UUID")
    ),
    responses(
        (status = 200, description = "Class definition deleted successfully"),
        (status = 400, description = "Cannot delete class definition with existing entities"),
        (status = 404, description = "Class definition not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt_auth" = [])
    )
)]
#[delete("/class-definitions/{uuid}")]
async fn delete_class_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
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

    // Check if table exists
    let table_exists = match repository.check_table_exists(&table_name).await {
        Ok(exists) => exists,
        Err(_) => false,
    };

    if table_exists {
        // Check if there's data in the table
        let count = match repository.count_table_records(&table_name).await {
            Ok(count) => count,
            Err(_) => 0,
        };

        if count > 0 {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Cannot delete class definition '{}' because there are {} entities using it", 
                    definition.entity_type, count)
            }));
        }
    }

    // Delete the class definition
    match repository.delete(&path.uuid).await {
        Ok(_) => {
            // Also clean up from entity registry
            let _ = repository
                .delete_from_entity_registry(&definition.entity_type)
                .await;

            HttpResponse::Ok().json(json!({
                "message": "Class definition deleted successfully",
                "note": format!("Table {} was not dropped for safety reasons. Drop it manually if needed.", table_name)
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to delete class definition: {}", e)
        })),
    }
}

/// Register class definition routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_class_definitions)
        .service(get_class_definition)
        .service(create_class_definition)
        .service(update_class_definition)
        .service(delete_class_definition);
}
