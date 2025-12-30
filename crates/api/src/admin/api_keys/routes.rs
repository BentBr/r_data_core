#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{delete, get, post, put, web, Responder};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

use crate::admin::api_keys::models::{
    ApiKeyCreatedResponse, ApiKeyResponse, CreateApiKeyRequest, ReassignApiKeyRequest,
};
use crate::admin::query_helpers::to_list_query_params;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::query::StandardQuery;
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_persistence::{ApiKeyRepository, ApiKeyRepositoryTrait};
use r_data_core_services::query_validation::FieldValidator;

/// Register API key routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_api_key)
        .service(list_api_keys)
        .service(revoke_api_key)
        .service(reassign_api_key);
}

/// List API keys for the authenticated user with pagination and sorting
#[utoipa::path(
    get,
    path = "/admin/api/v1/api-keys",
    tag = "api-keys",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Number of items per page (default: 20, max: 100, or -1 for unlimited)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return (alternative to per_page)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (alternative to page-based pagination)"),
        ("sort_by" = Option<String>, Query, description = "Field to sort by (e.g., name, is_active, last_used_at, created_at)"),
        ("sort_order" = Option<String>, Query, description = "Sort order: 'asc' or 'desc' (default: 'asc')")
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
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    query: web::Query<StandardQuery>,
) -> impl Responder {
    // Parse UUID first - if invalid, return 401 (authentication error)
    let user_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            return ApiResponse::<()>::unauthorized(&format!("Invalid UUID in auth token: {e}"));
        }
    };

    // Check permission after UUID is validated
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::ApiKeys,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to list API keys");
    }

    // Create field validator
    let pool = Arc::new(state.db_pool().clone());
    let field_validator = Arc::new(FieldValidator::new(pool));

    // Convert StandardQuery to ListQueryParams and use service method that handles all validation
    let params = to_list_query_params(&query);
    let service = state.api_key_service();
    match service
        .list_keys_for_user_with_query(user_uuid, &params, &field_validator)
        .await
    {
        Ok((rows, validated)) => {
            // Get total count
            let pool = Arc::new(state.db_pool().clone());
            let repo = ApiKeyRepository::new(pool);
            let total = repo.count_by_user(user_uuid).await.unwrap_or(0);

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

            ApiResponse::ok_paginated(api_keys, total, validated.page, validated.per_page)
        }
        Err(e) => {
            error!("Failed to list API keys: {e}");
            match e {
                r_data_core_core::error::Error::Validation(msg) => {
                    ApiResponse::<()>::bad_request(&msg)
                }
                _ => ApiResponse::<()>::internal_error("Failed to retrieve API keys"),
            }
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
    state: web::Data<ApiStateWrapper>,
    req: web::Json<CreateApiKeyRequest>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::ApiKeys,
        &PermissionType::Create,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to create API keys");
    }

    let pool = Arc::new(state.db_pool().clone());
    let creator_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            let sub = &auth.0.sub;
            error!("Invalid UUID in auth token: {sub} - {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication token");
        }
    };
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
            error!("Failed to check for existing API key: {e}");
            return ApiResponse::<()>::internal_error("Failed to check for existing API key");
        }
    }

    // Create the API key
    let description = req.description.clone().unwrap_or_default();
    #[allow(clippy::cast_possible_truncation)] // 365 days fits in i32
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
                error!("API key created but not found: {uuid}");
                ApiResponse::<()>::internal_error("API key created but not found")
            }
            Err(e) => {
                error!("Failed to retrieve created API key: {e}");
                ApiResponse::<()>::internal_error("Failed to retrieve created API key")
            }
        },
        Err(e) => {
            error!("Failed to create API key: {e}");
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
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    auth: RequiredAuth,
) -> impl Responder {
    // Parse UUID first - if invalid, return 401 (authentication error)
    let user_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            return ApiResponse::<()>::unauthorized(&format!("Invalid UUID in auth token: {e}"));
        }
    };

    // Check permission after UUID is validated
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::ApiKeys,
        &PermissionType::Delete,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to revoke API keys");
    }

    let pool = Arc::new(state.db_pool().clone());
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
                Ok(()) => ApiResponse::<()>::message("API key revoked successfully"),
                Err(e) => {
                    error!("Failed to revoke API key: {e}");
                    ApiResponse::<()>::internal_error("Failed to revoke API key")
                }
            }
        }
        Ok(None) => ApiResponse::<()>::not_found("API key"),
        Err(e) => {
            error!("Failed to retrieve API key: {e}");
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
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    req: web::Json<ReassignApiKeyRequest>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::ApiKeys,
        &PermissionType::Update,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to reassign API keys");
    }

    let pool = Arc::new(state.db_pool().clone());
    let user_uuid = match Uuid::parse_str(&auth.0.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            let sub = &auth.0.sub;
            error!("Invalid UUID in auth token: {sub} - {e}");
            return ApiResponse::<()>::internal_error("Invalid authentication token");
        }
    };
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
                Ok(Some(_)) => ApiResponse::<()>::conflict(
                    "An API key with this name already exists for the target user",
                ),
                Ok(None) => {
                    // Reassign the key
                    match repo.reassign(api_key_uuid, new_user_uuid).await {
                        Ok(()) => ApiResponse::<()>::message("API key reassigned successfully"),
                        Err(e) => {
                            error!("Failed to reassign API key: {e}");
                            ApiResponse::<()>::internal_error("Failed to reassign API key")
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to check for existing API key: {e}");
                    ApiResponse::<()>::internal_error("Failed to check for existing API key")
                }
            }
        }
        Ok(None) => ApiResponse::<()>::not_found("API key"),
        Err(e) => {
            error!("Failed to retrieve API key: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve API key")
        }
    }
}
