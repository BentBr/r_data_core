use actix_web::{delete, get, post, web, HttpResponse, Responder};
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::auth::AuthUserClaims;
use crate::api::ApiState;
use crate::entity::admin_user::ApiKey;

/// Register API key routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_api_key)
        .service(list_api_keys)
        .service(revoke_api_key);
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
    let user_uuid = Uuid::parse_str(&auth.sub).expect("Invalid UUID in auth token");

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
    let user_uuid = Uuid::parse_str(&auth.sub).expect("Invalid UUID in auth token");

    match sqlx::query!(
        r#"
        SELECT uuid, name, description, is_active, created_at, expires_at, last_used_at
        FROM api_keys 
        WHERE user_uuid = $1
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
    path: web::Path<Uuid>,
    auth: web::ReqData<AuthUserClaims>,
) -> impl Responder {
    let pool = &state.db_pool;
    let api_key_uuid = path.into_inner();
    let user_uuid = Uuid::parse_str(&auth.sub).expect("Invalid UUID in auth token");

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
