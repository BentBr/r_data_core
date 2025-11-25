#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{delete, get, post, put, web, Responder};
use log::error;
use uuid::Uuid;
use std::sync::Arc;

use crate::auth::auth_enum::RequiredAuth;
use crate::query::PaginationQuery;
use crate::response::ApiResponse;
use crate::api_state::ApiStateTrait;
use r_data_core_persistence::{ApiKeyRepository, ApiKeyRepositoryTrait};
use crate::admin::api_keys::models::{ApiKeyCreatedResponse, ApiKeyResponse, CreateApiKeyRequest, ReassignApiKeyRequest};

/// Register API key routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_api_key)
        .service(list_api_keys)
        .service(revoke_api_key)
        .service(reassign_api_key);
}

/// List API keys for the authenticated user with pagination
#[utoipa::path(
    get,
    path = "/admin/api/v1/api-keys",
    tag = "api-keys",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Number of items per page (default: 20, max: 100)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return (alternative to per_page)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (alternative to page-based pagination)")
    ),
    responses(
        (status = 200, description = "List of API keys with pagination", body = Vec<ApiKeyResponse>),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("")]
pub async fn list_api_keys(
    state: web::Data<dyn ApiStateTrait>,
    auth: RequiredAuth,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    let pool = Arc::new(state.db_pool().clone());
    let user_uuid = Uuid::parse_str(&auth.0.sub).expect("Invalid UUID in auth token");

    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    let repo = ApiKeyRepository::new(pool);

    // Get both the API keys and the total count
    let (api_keys_result, count_result) = tokio::join!(
        repo.list_by_user(user_uuid, limit, offset),
        repo.count_by_user(user_uuid)
    );

    match (api_keys_result, count_result) {
        (Ok(rows), Ok(total)) => {
            let api_keys: Vec<ApiKeyResponse> = rows
                .into_iter()
                .map(|row| ApiKeyResponse {
                    uuid: row.uuid,
                    name: row.name.clone(),
                    description: row.description.clone(),
                    is_active: row.is_active,
                    created_at: row.created_at,
                    expires_at: row.expires_at,
                    last_used_at: row.last_used_at,
                    created_by: row.created_by,
                    user_uuid: row.user_uuid,
                    published: row.published,
                })
                .collect::<Vec<_>>();

            ApiResponse::ok_paginated(api_keys, total, page, per_page)
        }
        (Err(e), _) => {
            error!("Failed to list API keys: {}", e);
            ApiResponse::<()>::internal_error("Failed to retrieve API keys")
        }
        (_, Err(e)) => {
            error!("Failed to count API keys: {}", e);
            ApiResponse::<()>::internal_error("Failed to count API keys")
        }
    }
}

/// Create a new API key for the authenticated user
#[utoipa::path(
    post,
    path = "/admin/api/v1/api-keys",
    tag = "api-keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created successfully", body = ApiKeyCreatedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Conflict - API key name already exists"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("")]
pub async fn create_api_key(
    state: web::Data<dyn ApiStateTrait>,
    req: web::Json<CreateApiKeyRequest>,
    auth: RequiredAuth,
) -> impl Responder {
    let pool = Arc::new(state.db_pool().clone());
    let creator_uuid = Uuid::parse_str(&auth.0.sub).expect("Invalid UUID in auth token");
    let user_uuid = creator_uuid; // Assign to the authenticated user by default

    // First, check if the user already has a key with this name
    let repo = ApiKeyRepository::new(pool);

    match repo.get_by_name(user_uuid, &req.name).await {
        Ok(Some(_)) => {
            return ApiResponse::<()>::conflict(
                "An API key with this name already exists for this user",
            );
        }
        Ok(None) => {
            // Proceed with creation
        }
        Err(e) => {
            error!("Failed to check for existing API key: {}", e);
            return ApiResponse::<()>::internal_error("Failed to check for existing API key");
        }
    }

    // Create the API key
    let description = req.description.clone().unwrap_or_default();
    let expires_in_days = req.expires_in_days.unwrap_or(365) as i32;

    match repo
        .create_new_api_key(&req.name, &description, creator_uuid, expires_in_days)
        .await
    {
        Ok((uuid, api_key)) => match repo.get_by_uuid(uuid).await {
            Ok(Some(key)) => {
                let response: ApiKeyCreatedResponse = ApiKeyCreatedResponse {
                    uuid: key.uuid,
                    name: key.name.clone(),
                    api_key,
                    description: key.description.clone(),
                    is_active: key.is_active,
                    created_at: key.created_at,
                    expires_at: key.expires_at,
                    created_by: key.created_by,
                    user_uuid: key.user_uuid,
                    published: key.published,
                    last_used_at: key.last_used_at,
                };
                ApiResponse::<ApiKeyCreatedResponse>::created(response)
            }
            Ok(None) => {
                error!("API key created but not found: {}", uuid);
                ApiResponse::<()>::internal_error("API key created but not found")
            }
            Err(e) => {
                error!("Failed to retrieve created API key: {}", e);
                ApiResponse::<()>::internal_error("Failed to retrieve created API key")
            }
        },
        Err(e) => {
            error!("Failed to create API key: {}", e);
            ApiResponse::<()>::internal_error("Failed to create API key")
        }
    }
}

/// Revoke an API key
#[utoipa::path(
    delete,
    path = "/admin/api/v1/api-keys/{uuid}",
    tag = "api-keys",
    params(
        ("uuid" = Uuid, Path, description = "UUID of the API key to revoke")
    ),
    responses(
        (status = 200, description = "API key revoked successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - API key does not belong to user"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{uuid}")]
pub async fn revoke_api_key(
    state: web::Data<dyn ApiStateTrait>,
    path: web::Path<Uuid>,
    auth: RequiredAuth,
) -> impl Responder {
    let pool = Arc::new(state.db_pool().clone());
    let user_uuid = Uuid::parse_str(&auth.0.sub).expect("Invalid UUID in auth token");
    let api_key_uuid = path.into_inner();

    let repo = ApiKeyRepository::new(pool);

    // First, check if the API key exists and belongs to the user
    match repo.get_by_uuid(api_key_uuid).await {
        Ok(Some(key)) => {
            if key.user_uuid != user_uuid {
                return ApiResponse::<()>::forbidden(
                    "You don't have permission to revoke this API key",
                );
            }

            // Revoke the key
            match repo.revoke(api_key_uuid).await {
                Ok(_) => ApiResponse::<()>::message("API key revoked successfully"),
                Err(e) => {
                    error!("Failed to revoke API key: {}", e);
                    ApiResponse::<()>::internal_error("Failed to revoke API key")
                }
            }
        }
        Ok(None) => ApiResponse::<()>::not_found("API key"),
        Err(e) => {
            error!("Failed to retrieve API key: {}", e);
            ApiResponse::<()>::internal_error("Failed to retrieve API key")
        }
    }
}

/// Reassign an API key to another user
#[utoipa::path(
    put,
    path = "/admin/api/v1/api-keys/{uuid}/reassign",
    tag = "api-keys",
    params(
        ("uuid" = Uuid, Path, description = "UUID of the API key to reassign")
    ),
    request_body = ReassignApiKeyRequest,
    responses(
        (status = 200, description = "API key reassigned successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - API key does not belong to user"),
        (status = 404, description = "API key not found"),
        (status = 409, description = "User already has an API key with this name"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[put("/{uuid}/reassign")]
pub async fn reassign_api_key(
    state: web::Data<dyn ApiStateTrait>,
    path: web::Path<Uuid>,
    req: web::Json<ReassignApiKeyRequest>,
    auth: RequiredAuth,
) -> impl Responder {
    let pool = Arc::new(state.db_pool().clone());
    let user_uuid = Uuid::parse_str(&auth.0.sub).expect("Invalid UUID in auth token");
    let api_key_uuid = path.into_inner();
    let new_user_uuid = req.user_uuid;

    let repo = ApiKeyRepository::new(pool);

    // First, check if the API key exists and belongs to the user
    match repo.get_by_uuid(api_key_uuid).await {
        Ok(Some(key)) => {
            if key.user_uuid != user_uuid {
                return ApiResponse::<()>::forbidden(
                    "You don't have permission to reassign this API key",
                );
            }

            // Check if the key with the same name already exists for the new user
            match repo.get_by_name(new_user_uuid, &key.name).await {
                Ok(Some(_)) => {
                    return ApiResponse::<()>::conflict(
                        "An API key with this name already exists for the target user",
                    );
                }
                Ok(None) => {
                    // Reassign the key
                    match repo.reassign(api_key_uuid, new_user_uuid).await {
                        Ok(_) => ApiResponse::<()>::message("API key reassigned successfully"),
                        Err(e) => {
                            error!("Failed to reassign API key: {}", e);
                            ApiResponse::<()>::internal_error("Failed to reassign API key")
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to check for existing API key: {}", e);
                    ApiResponse::<()>::internal_error("Failed to check for existing API key")
                }
            }
        }
        Ok(None) => ApiResponse::<()>::not_found("API key"),
        Err(e) => {
            error!("Failed to retrieve API key: {}", e);
            ApiResponse::<()>::internal_error("Failed to retrieve API key")
        }
    }
}

