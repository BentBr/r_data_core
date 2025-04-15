use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::api::auth::AuthUserClaims;
use crate::api::middleware::JwtAuth;
use crate::api::ApiState;
use crate::db::PgPoolExtension;
use crate::entity::admin_user::ApiKey;
use crate::entity::{AdminUser, ClassDefinition, PermissionScheme, WorkflowEntity};
use log::{error, info};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

// Import constants from the lib.rs
use crate::DESCRIPTION;
use crate::NAME;
use crate::VERSION;

/// Register admin API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/api/v1")
            .wrap(JwtAuth::new())
            .service(get_current_user)
            .service(list_class_definitions)
            .service(get_class_definition)
            .service(create_class_definition)
            .service(update_class_definition)
            .service(delete_class_definition)
            .service(list_workflows)
            .service(get_workflow)
            .service(create_workflow)
            .service(update_workflow)
            .service(delete_workflow)
            .service(list_permission_schemes)
            .service(get_permission_scheme)
            .service(create_permission_scheme)
            .service(update_permission_scheme)
            .service(delete_permission_scheme)
            .service(get_system_info)
            .service(create_api_key)
            .service(list_api_keys)
            .service(revoke_api_key),
    );
}

/// Get currently logged in user
#[get("/user")]
async fn get_current_user(
    data: web::Data<ApiState>,
    user_uuid: web::ReqData<Uuid>,
) -> impl Responder {
    let db_pool = &data.db_pool;
    let user_uuid = user_uuid.into_inner();

    let user_result = db_pool
        .repository_with_table::<AdminUser>("admin_users")
        .get_by_uuid(&user_uuid)
        .await;

    match user_result {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Could not retrieve user data"
        })),
    }
}

/// List class definitions
#[get("/class-definitions")]
async fn list_class_definitions(
    data: web::Data<ApiState>,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let result = db_pool
        .repository_with_table::<ClassDefinition>("class_definitions")
        .list(None, Some("entity_type ASC"), Some(limit), Some(offset))
        .await;

    match result {
        Ok(definitions) => HttpResponse::Ok().json(definitions),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to list class definitions: {}", e)
        })),
    }
}

/// Get a class definition by UUID
#[get("/class-definitions/{uuid}")]
async fn get_class_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    let result = db_pool
        .repository_with_table::<ClassDefinition>("class_definitions")
        .get_by_uuid(&path.uuid)
        .await;

    match result {
        Ok(definition) => HttpResponse::Ok().json(definition),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Class definition not found"
        })),
    }
}

/// Create a new class definition
#[post("/class-definitions")]
async fn create_class_definition(
    data: web::Data<ApiState>,
    definition: web::Json<ClassDefinition>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    // Save class definition
    let result = db_pool
        .repository_with_table::<ClassDefinition>("class_definitions")
        .create(&definition)
        .await;

    match result {
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
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to generate SQL schema: {}", e)
                    }));
                }
            };

            let apply_result = sqlx::query(&schema_sql).execute(db_pool).await;

            match apply_result {
                Ok(_) => HttpResponse::Created().json(serde_json::json!({
                    "uuid": uuid,
                    "message": "Class definition created successfully with database table"
                })),
                Err(e) => {
                    // Table creation failed, but definition was saved
                    HttpResponse::Ok().json(serde_json::json!({
                        "uuid": uuid,
                        "warning": format!("Class definition created but table creation failed: {}", e),
                        "message": "Class definition was saved but will not be usable until the table is created"
                    }))
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to create class definition: {}", e)
        })),
    }
}

/// Update a class definition
#[put("/class-definitions/{uuid}")]
async fn update_class_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    definition: web::Json<ClassDefinition>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    // Update class definition
    let result = db_pool
        .repository_with_table::<ClassDefinition>("class_definitions")
        .update(&path.uuid, &definition)
        .await;

    match result {
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
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to generate SQL schema: {}", e)
                    }));
                }
            };

            let apply_result = sqlx::query(&schema_sql).execute(db_pool).await;

            match apply_result {
                Ok(_) => {
                    HttpResponse::Ok().json(serde_json::json!({
                        "message": "Class definition and database table updated successfully"
                    }))
                },
                Err(e) => {
                    HttpResponse::Ok().json(serde_json::json!({
                        "warning": format!("Class definition updated but table update failed: {}", e),
                        "message": "Class definition was saved but table structure may not match the definition"
                    }))
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to update class definition: {}", e)
        })),
    }
}

/// Delete a class definition
#[delete("/class-definitions/{uuid}")]
async fn delete_class_definition(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    // First get the class definition to know its table name
    let definition_result = db_pool
        .repository_with_table::<ClassDefinition>("class_definitions")
        .get_by_uuid(&path.uuid)
        .await;

    let definition = match definition_result {
        Ok(def) => def,
        Err(_) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Class definition not found"
            }));
        }
    };

    // Check if there are any entities of this type
    let definition: ClassDefinition = definition.clone();
    let table_name = definition.get_table_name();

    // Check if table exists
    let table_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_schema = 'public' 
            AND table_name = $1
        )",
    )
    .bind(table_name.to_lowercase())
    .fetch_one(db_pool)
    .await
    .unwrap_or((false,));

    if table_exists.0 {
        // Check if there's data in the table
        let has_data: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table_name))
            .fetch_one(db_pool)
            .await
            .unwrap_or((0,));

        if has_data.0 > 0 {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Cannot delete class definition '{}' because there are {} entities using it", 
                    definition.entity_type, has_data.0)
            }));
        }
    }

    // Delete the class definition
    let result = db_pool
        .repository_with_table::<ClassDefinition>("class_definitions")
        .delete(&path.uuid)
        .await;

    match result {
        Ok(_) => {
            // Also clean up from entity registry
            let _ = sqlx::query("DELETE FROM entities WHERE name = $1")
                .bind(&definition.entity_type)
                .execute(db_pool)
                .await;

            // Don't drop the table automatically - it's too risky
            // The admin should do this manually if needed

            HttpResponse::Ok().json(serde_json::json!({
                "message": "Class definition deleted successfully",
                "note": format!("Table {} was not dropped for safety reasons. Drop it manually if needed.", table_name)
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to delete class definition: {}", e)
        })),
    }
}

/// List workflows
#[get("/workflows")]
async fn list_workflows(
    data: web::Data<ApiState>,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let result = db_pool
        .repository_with_table::<WorkflowEntity>("workflows")
        .list(None, Some("name ASC"), Some(limit), Some(offset))
        .await;

    match result {
        Ok(workflows) => HttpResponse::Ok().json(workflows),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to list workflows: {}", e)
        })),
    }
}

/// Get a workflow by UUID
#[get("/workflows/{uuid}")]
async fn get_workflow(data: web::Data<ApiState>, path: web::Path<PathUuid>) -> impl Responder {
    let db_pool = &data.db_pool;

    let result = db_pool
        .repository_with_table::<WorkflowEntity>("workflows")
        .get_by_uuid(&path.uuid)
        .await;

    match result {
        Ok(workflow) => HttpResponse::Ok().json(workflow),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Workflow not found"
        })),
    }
}

/// Create a new workflow
#[post("/workflows")]
async fn create_workflow(
    data: web::Data<ApiState>,
    workflow: web::Json<WorkflowEntity>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    // Validate workflow before saving
    if let Err(e) = workflow.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Invalid workflow: {}", e)
        }));
    }

    let result = db_pool
        .repository_with_table::<WorkflowEntity>("workflows")
        .create(&workflow)
        .await;

    match result {
        Ok(uuid) => HttpResponse::Created().json(serde_json::json!({
            "uuid": uuid,
            "message": "Workflow created successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to create workflow: {}", e)
        })),
    }
}

/// Update a workflow
#[put("/workflows/{uuid}")]
async fn update_workflow(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    workflow: web::Json<WorkflowEntity>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    // Validate workflow before saving
    if let Err(e) = workflow.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Invalid workflow: {}", e)
        }));
    }

    let result = db_pool
        .repository_with_table::<WorkflowEntity>("workflows")
        .update(&path.uuid, &workflow)
        .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Workflow updated successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to update workflow: {}", e)
        })),
    }
}

/// Delete a workflow
#[delete("/workflows/{uuid}")]
async fn delete_workflow(data: web::Data<ApiState>, path: web::Path<PathUuid>) -> impl Responder {
    let db_pool = &data.db_pool;

    let result = db_pool
        .repository_with_table::<WorkflowEntity>("workflows")
        .delete(&path.uuid)
        .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Workflow deleted successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to delete workflow: {}", e)
        })),
    }
}

/// List permission schemes
#[get("/permission-schemes")]
async fn list_permission_schemes(
    data: web::Data<ApiState>,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let result = db_pool
        .repository_with_table::<PermissionScheme>("permission_schemes")
        .list(None, Some("name ASC"), Some(limit), Some(offset))
        .await;

    match result {
        Ok(schemes) => HttpResponse::Ok().json(schemes),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to list permission schemes: {}", e)
        })),
    }
}

/// Get a permission scheme by UUID
#[get("/permission-schemes/{uuid}")]
async fn get_permission_scheme(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    let result = db_pool
        .repository_with_table::<PermissionScheme>("permission_schemes")
        .get_by_uuid(&path.uuid)
        .await;

    match result {
        Ok(scheme) => HttpResponse::Ok().json(scheme),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Permission scheme not found"
        })),
    }
}

/// Create a new permission scheme
#[post("/permission-schemes")]
async fn create_permission_scheme(
    data: web::Data<ApiState>,
    scheme: web::Json<PermissionScheme>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    let result = db_pool
        .repository_with_table::<PermissionScheme>("permission_schemes")
        .create(&scheme)
        .await;

    match result {
        Ok(uuid) => HttpResponse::Created().json(serde_json::json!({
            "uuid": uuid,
            "message": "Permission scheme created successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to create permission scheme: {}", e)
        })),
    }
}

/// Update a permission scheme
#[put("/permission-schemes/{uuid}")]
async fn update_permission_scheme(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    scheme: web::Json<PermissionScheme>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    // Check if this is a system scheme
    let existing_scheme = db_pool
        .repository_with_table::<PermissionScheme>("permission_schemes")
        .get_by_uuid(&path.uuid)
        .await;

    if let Ok(existing) = existing_scheme {
        if existing.is_system {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "error": "Cannot modify a system permission scheme"
            }));
        }
    }

    let result = db_pool
        .repository_with_table::<PermissionScheme>("permission_schemes")
        .update(&path.uuid, &scheme)
        .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Permission scheme updated successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to update permission scheme: {}", e)
        })),
    }
}

/// Delete a permission scheme
#[delete("/permission-schemes/{uuid}")]
async fn delete_permission_scheme(
    data: web::Data<ApiState>,
    path: web::Path<PathUuid>,
) -> impl Responder {
    let db_pool = &data.db_pool;

    // Check if this is a system scheme
    let existing_scheme = db_pool
        .repository_with_table::<PermissionScheme>("permission_schemes")
        .get_by_uuid(&path.uuid)
        .await;

    if let Ok(existing) = existing_scheme {
        if existing.is_system {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "error": "Cannot delete a system permission scheme"
            }));
        }
    }

    let result = db_pool
        .repository_with_table::<PermissionScheme>("permission_schemes")
        .delete(&path.uuid)
        .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Permission scheme deleted successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to delete permission scheme: {}", e)
        })),
    }
}

/// Get system information
#[get("/system/info")]
async fn get_system_info(data: web::Data<ApiState>) -> impl Responder {
    let _db_pool = &data.db_pool;
    let _cache_manager = &data.cache_manager;

    // Build system information response
    let system_info = serde_json::json!({
        "name": NAME.to_string(),
        "version": VERSION.to_string(),
        "description": DESCRIPTION.to_string(),
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "os": std::env::consts::OS.to_string(),
        "arch": std::env::consts::ARCH.to_string(),
    });

    HttpResponse::Ok().json(system_info)
}

/// Pagination query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Path parameter for UUID
#[derive(Debug, Deserialize, ToSchema)]
pub struct PathUuid {
    pub uuid: Uuid,
}

// API Keys
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyResponse {
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
    pub last_used_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyCreatedResponse {
    pub uuid: Uuid,
    pub name: String,
    pub api_key: String, // Only returned once when created
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
}

/// Create a new API key for the authenticated user
#[post("/api-keys")]
pub async fn create_api_key(
    state: web::Data<ApiState>,
    req: web::Json<CreateApiKeyRequest>,
    auth: web::ReqData<AuthUserClaims>,
) -> impl Responder {
    let pool = &state.db_pool;
    let user_uuid = Uuid::from_u128(auth.sub as u128);

    // Calculate expiration date if provided
    let expires_at = req
        .expires_in_days
        .map(|days| OffsetDateTime::now_utc() + time::Duration::days(days));

    // Create new API key
    let api_key = ApiKey::new(
        user_uuid,
        req.name.clone(),
        req.description.clone(),
        expires_at,
    );

    // Store API key
    let api_key_value = api_key.api_key.clone();

    match sqlx::query!(
        r#"
        INSERT INTO api_keys 
        (user_uuid, key_hash, name, description, is_active, created_at, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING uuid, created_at
        "#,
        user_uuid,
        api_key.api_key,
        api_key.name,
        api_key.description,
        api_key.is_active,
        api_key.created_at,
        api_key.expires_at
    )
    .fetch_one(pool)
    .await
    {
        Ok(row) => {
            // Return the created API key with the key value (only shown once)
            HttpResponse::Created().json(ApiKeyCreatedResponse {
                uuid: row.uuid,
                name: req.name.clone(),
                api_key: api_key_value,
                description: req.description.clone(),
                is_active: true,
                created_at: row.created_at,
                expires_at,
            })
        }
        Err(e) => {
            error!("Failed to create API key: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to create API key"
            }))
        }
    }
}

/// List all API keys for the authenticated user
#[get("/api-keys")]
pub async fn list_api_keys(
    state: web::Data<ApiState>,
    auth: web::ReqData<AuthUserClaims>,
) -> impl Responder {
    let pool = &state.db_pool;
    let user_uuid = Uuid::from_u128(auth.sub as u128);

    match sqlx::query!(
        r#"
        SELECT uuid, name, description, is_active, created_at, expires_at, last_used_at
        FROM api_keys 
        WHERE uuid = $1
        ORDER BY created_at DESC
        "#,
        user_uuid
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => {
            let api_keys: Vec<ApiKeyResponse> = rows
                .iter()
                .map(|row| ApiKeyResponse {
                    uuid: row.uuid,
                    name: row.name.clone(),
                    description: row.description.clone(),
                    is_active: row.is_active,
                    created_at: row.created_at,
                    expires_at: row.expires_at,
                    last_used_at: row.last_used_at,
                })
                .collect();

            HttpResponse::Ok().json(api_keys)
        }
        Err(e) => {
            error!("Failed to list API keys: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to retrieve API keys"
            }))
        }
    }
}

/// Revoke an API key
#[delete("/api-keys/{uuid}")]
pub async fn revoke_api_key(
    state: web::Data<ApiState>,
    path: web::Path<PathUuid>,
    auth: web::ReqData<AuthUserClaims>,
) -> impl Responder {
    let pool = &state.db_pool;
    let api_key_uuid = path.uuid;
    let user_uuid = Uuid::from_u128(auth.sub as u128);

    // Verify the API key belongs to the authenticated user
    match sqlx::query!(
        "SELECT uuid FROM api_keys WHERE uuid = $1 AND user_uuid = $2",
        api_key_uuid,
        user_uuid
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some(_)) => {
            // Deactivate the API key
            match sqlx::query!(
                "UPDATE api_keys SET is_active = FALSE WHERE uuid = $1",
                api_key_uuid
            )
            .execute(pool)
            .await
            {
                Ok(_) => HttpResponse::Ok().json(json!({
                    "message": "API key revoked successfully"
                })),
                Err(e) => {
                    error!("Failed to revoke API key: {}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "error": "Failed to revoke API key"
                    }))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error": "API key not found or does not belong to you"
        })),
        Err(e) => {
            error!("Database error: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Internal server error"
            }))
        }
    }
}
