#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, put, web, Responder};

use crate::admin::system::models::{EntityVersioningSettingsDto, UpdateSettingsBody};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::ApiResponse;
use r_data_core_core::permissions::permission_scheme::{PermissionType, ResourceNamespace};
use r_data_core_services::SettingsService;

/// Register system routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_entity_versioning_settings);
    cfg.service(update_entity_versioning_settings);
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/system/settings/entity-versioning",
    tag = "system",
    responses(
        (status = 200, description = "Get entity versioning settings", body = EntityVersioningSettingsDto),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/settings/entity-versioning")]
pub async fn get_entity_versioning_settings(
    data: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view system settings");
    }

    let service = SettingsService::new(data.db_pool().clone(), data.cache_manager().clone());
    match service.get_entity_versioning_settings().await {
        Ok(settings) => ApiResponse::ok(EntityVersioningSettingsDto::from(settings)),
        Err(e) => {
            log::error!("Failed to load settings: {}", e);
            ApiResponse::<()>::internal_error("Failed to load settings")
        }
    }
}

#[utoipa::path(
    put,
    path = "/admin/api/v1/system/settings/entity-versioning",
    tag = "system",
    request_body = UpdateSettingsBody,
    responses(
        (status = 200, description = "Updated entity versioning settings", body = EntityVersioningSettingsDto),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[put("/settings/entity-versioning")]
pub async fn update_entity_versioning_settings(
    data: web::Data<ApiStateWrapper>,
    body: web::Json<UpdateSettingsBody>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Update,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to update system settings");
    }

    let service = SettingsService::new(data.db_pool().clone(), data.cache_manager().clone());

    // Merge with current to allow partial updates
    let mut current = match service.get_entity_versioning_settings().await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to read current settings: {}", e);
            return ApiResponse::<()>::internal_error("Failed to read current settings");
        }
    };

    if let Some(v) = body.enabled {
        current.enabled = v;
    }
    if body.max_versions.is_some() {
        current.max_versions = body.max_versions;
    }
    if body.max_age_days.is_some() {
        current.max_age_days = body.max_age_days;
    }

    // Determine user performing the update
    let updated_by = match auth.user_uuid() {
        Some(u) => u,
        None => {
            return ApiResponse::<()>::internal_error("No authentication claims found for update")
        }
    };

    match service
        .update_entity_versioning_settings(&current, updated_by)
        .await
    {
        Ok(()) => ApiResponse::ok(EntityVersioningSettingsDto::from(current)),
        Err(e) => {
            log::error!("Failed to update settings: {}", e);
            ApiResponse::<()>::internal_error("Failed to update settings")
        }
    }
}
