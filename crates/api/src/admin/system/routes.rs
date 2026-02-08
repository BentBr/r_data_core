#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::admin::system::models::{
    ComponentVersionDto, EntityVersioningSettingsDto, LicenseStatusDto, LicenseVerificationRequest,
    LicenseVerificationResponse, SystemVersionsDto, UpdateSettingsBody,
    UpdateWorkflowRunLogSettingsBody, WorkflowRunLogSettingsDto,
};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::ApiResponse;
use actix_web::{get, post, put, web, Responder};
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_persistence::ComponentVersionRepository;
use r_data_core_services::SettingsService;

/// Core version from Cargo.toml
const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Register system routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_entity_versioning_settings);
    cfg.service(update_entity_versioning_settings);
    cfg.service(get_workflow_run_log_settings);
    cfg.service(update_workflow_run_log_settings);
    cfg.service(get_license_status);
    cfg.service(get_system_versions);
    // Internal endpoint (not in Swagger)
    cfg.service(verify_license_internal);
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
            log::error!("Failed to load settings: {e}");
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
            log::error!("Failed to read current settings: {e}");
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
    let Some(updated_by) = auth.user_uuid() else {
        return ApiResponse::<()>::internal_error("No authentication claims found for update");
    };

    match service
        .update_entity_versioning_settings(&current, updated_by)
        .await
    {
        Ok(()) => ApiResponse::ok(EntityVersioningSettingsDto::from(current)),
        Err(e) => {
            log::error!("Failed to update settings: {e}");
            ApiResponse::<()>::internal_error("Failed to update settings")
        }
    }
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/system/settings/workflow-run-logs",
    tag = "system",
    responses(
        (status = 200, description = "Get workflow run log settings", body = WorkflowRunLogSettingsDto),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/settings/workflow-run-logs")]
pub async fn get_workflow_run_log_settings(
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
    match service.get_workflow_run_log_settings().await {
        Ok(settings) => ApiResponse::ok(WorkflowRunLogSettingsDto::from(settings)),
        Err(e) => {
            log::error!("Failed to load workflow run log settings: {e}");
            ApiResponse::<()>::internal_error("Failed to load settings")
        }
    }
}

#[utoipa::path(
    put,
    path = "/admin/api/v1/system/settings/workflow-run-logs",
    tag = "system",
    request_body = UpdateWorkflowRunLogSettingsBody,
    responses(
        (status = 200, description = "Updated workflow run log settings", body = WorkflowRunLogSettingsDto),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[put("/settings/workflow-run-logs")]
pub async fn update_workflow_run_log_settings(
    data: web::Data<ApiStateWrapper>,
    body: web::Json<UpdateWorkflowRunLogSettingsBody>,
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
    let mut current = match service.get_workflow_run_log_settings().await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to read current workflow run log settings: {e}");
            return ApiResponse::<()>::internal_error("Failed to read current settings");
        }
    };

    if let Some(v) = body.enabled {
        current.enabled = v;
    }
    if body.max_runs.is_some() {
        current.max_runs = body.max_runs;
    }
    if body.max_age_days.is_some() {
        current.max_age_days = body.max_age_days;
    }

    // Determine user performing the update
    let Some(updated_by) = auth.user_uuid() else {
        return ApiResponse::<()>::internal_error("No authentication claims found for update");
    };

    match service
        .update_workflow_run_log_settings(&current, updated_by)
        .await
    {
        Ok(()) => ApiResponse::ok(WorkflowRunLogSettingsDto::from(current)),
        Err(e) => {
            log::error!("Failed to update workflow run log settings: {e}");
            ApiResponse::<()>::internal_error("Failed to update settings")
        }
    }
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/system/license",
    tag = "system",
    responses(
        (status = 200, description = "Get license status (returns cached result)", body = LicenseStatusDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/license")]
pub async fn get_license_status(
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
        return ApiResponse::<()>::forbidden("Insufficient permissions to view license status");
    }

    // Use license service from API state
    let license_service = data.license_service();

    match license_service.verify_license().await {
        Ok(result) => ApiResponse::ok(LicenseStatusDto::from(result)),
        Err(e) => {
            log::error!("Failed to retrieve license status: {e}");
            ApiResponse::<()>::internal_error("Failed to retrieve license status")
        }
    }
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/system/versions",
    tag = "system",
    responses(
        (status = 200, description = "Get system component versions", body = SystemVersionsDto),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/versions")]
pub async fn get_system_versions(
    data: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission - any authenticated user can view versions
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view system versions");
    }

    let repo = ComponentVersionRepository::new(data.db_pool().clone());

    // Fetch component versions from database
    let components = match repo.get_all().await {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to fetch component versions: {e}");
            return ApiResponse::<()>::internal_error("Failed to fetch component versions");
        }
    };

    // Build response
    let worker = components
        .iter()
        .find(|c| c.component_name == "worker")
        .map(|c| ComponentVersionDto {
            name: c.component_name.clone(),
            version: c.version.clone(),
            last_seen_at: c.last_seen_at,
        });

    let maintenance = components
        .iter()
        .find(|c| c.component_name == "maintenance")
        .map(|c| ComponentVersionDto {
            name: c.component_name.clone(),
            version: c.version.clone(),
            last_seen_at: c.last_seen_at,
        });

    ApiResponse::ok(SystemVersionsDto {
        core: CORE_VERSION.to_string(),
        worker,
        maintenance,
    })
}

/// Internal license verification endpoint (not documented in Swagger)
///
/// This endpoint allows an instance to verify license keys against itself,
/// enabling self-hosted license verification.
///
/// This endpoint is only active if both `LICENSE_PRIVATE_KEY` and `LICENSE_PUBLIC_KEY` are set.
///
/// Path: POST /admin/api/v1/system/internal/license/verify
#[post("/internal/license/verify")]
pub async fn verify_license_internal(
    data: web::Data<ApiStateWrapper>,
    body: web::Json<LicenseVerificationRequest>,
) -> impl Responder {
    use r_data_core_license::verify_license_key;

    // Get license config from the license service (uses global config)
    let license_service = data.license_service();
    let license_config = &license_service.config;

    // Endpoint is only active if both private and public keys are configured
    // Return 404 if keys are not set (endpoint doesn't exist)
    let Some((_pk, public_key)) = license_config
        .private_key
        .as_ref()
        .zip(license_config.public_key.as_ref())
    else {
        // If keys are not configured - the endpoint doesn't exist (404)
        return ApiResponse::<()>::not_found("License verification");
    };

    // Both keys are set - use public key for verification
    let verification_result = match verify_license_key(&body.license_key, public_key) {
        Ok(_claims) => LicenseVerificationResponse {
            valid: true,
            message: None,
        },
        Err(e) => LicenseVerificationResponse {
            valid: false,
            message: Some(format!("Invalid license key: {e}")),
        },
    };

    ApiResponse::ok(verification_result)
}
