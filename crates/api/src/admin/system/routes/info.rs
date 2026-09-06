#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::admin::system::models::{
    CapabilitiesResponse, ComponentVersionDto, LicenseStatusDto, LicenseVerificationRequest,
    LicenseVerificationResponse, SystemLogDto, SystemLogQuery, SystemVersionsDto,
};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::ApiResponse;
use actix_web::{get, post, web, Responder};
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_persistence::ComponentVersionRepository;
use r_data_core_persistence::{SystemLogRepository, SystemLogRepositoryTrait};
use time::format_description::well_known::Rfc3339;

/// Core version from Cargo.toml
const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[utoipa::path(
    get,
    path = "/admin/api/v1/system/capabilities",
    tag = "system",
    responses(
        (status = 200, description = "Get system capabilities (public, no auth required)", body = CapabilitiesResponse),
    ),
    security() // No authentication required — only exposes feature flags, no secrets
)]
#[get("/capabilities")]
pub async fn get_capabilities(data: web::Data<ApiStateWrapper>) -> impl Responder {
    let system_mail_configured = data.password_reset_service().is_some();
    let workflow_mail_configured = data.workflow_service().mail_service.is_some();

    let oidc = data.oidc().filter(|oidc| oidc.flow().is_configured());

    ApiResponse::ok(CapabilitiesResponse {
        system_mail_configured,
        workflow_mail_configured,
        oidc_enabled: oidc.is_some(),
        oidc_provider_name: oidc.and_then(|o| o.runtime().config().provider_name.clone()),
    })
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/system/logs",
    tag = "system",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("page_size" = Option<i64>, Query, description = "Items per page (default: 20, max: 100)"),
        ("log_type" = Option<String>, Query, description = "Filter by log type"),
        ("resource_type" = Option<String>, Query, description = "Filter by resource type"),
        ("status" = Option<String>, Query, description = "Filter by status")
    ),
    responses(
        (status = 200, description = "Paginated list of system logs", body = [SystemLogDto]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/logs")]
pub async fn list_system_logs(
    data: web::Data<ApiStateWrapper>,
    query: web::Query<SystemLogQuery>,
    auth: RequiredAuth,
) -> impl Responder {
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view system logs");
    }

    let (limit, offset, page, per_page) = query.to_pagination();
    let repo = SystemLogRepository::new(data.db_pool().clone());

    let resource_uuid_parsed = query
        .resource_uuid
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    let date_from_parsed = query
        .date_from
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| time::OffsetDateTime::parse(s, &Rfc3339).ok());

    let date_to_parsed = query
        .date_to
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| time::OffsetDateTime::parse(s, &Rfc3339).ok());

    let filter = r_data_core_persistence::SystemLogFilter {
        log_type: query.log_type.clone(),
        resource_type: query.resource_type.clone(),
        status: query.status.clone(),
        resource_uuid: resource_uuid_parsed,
        date_from: date_from_parsed,
        date_to: date_to_parsed,
    };

    match repo.list_paginated(limit, offset, &filter).await {
        Ok((logs, total)) => {
            let dtos: Vec<SystemLogDto> = logs.into_iter().map(SystemLogDto::from).collect();
            ApiResponse::ok_paginated(dtos, total, page, per_page)
        }
        Err(e) => {
            log::error!("Failed to list system logs: {e}");
            ApiResponse::<()>::internal_error("Failed to list system logs")
        }
    }
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/system/logs/{uuid}",
    tag = "system",
    params(("uuid" = uuid::Uuid, Path, description = "System log UUID")),
    responses(
        (status = 200, description = "System log entry", body = SystemLogDto),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/logs/{uuid}")]
pub async fn get_system_log(
    data: web::Data<ApiStateWrapper>,
    path: web::Path<uuid::Uuid>,
    auth: RequiredAuth,
) -> impl Responder {
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view system logs");
    }

    let uuid = path.into_inner();
    let repo = SystemLogRepository::new(data.db_pool().clone());

    match repo.get_by_uuid(uuid).await {
        Ok(Some(log)) => ApiResponse::ok(SystemLogDto::from(log)),
        Ok(None) => ApiResponse::<()>::not_found("System log not found"),
        Err(e) => {
            log::error!("Failed to get system log {uuid}: {e}");
            ApiResponse::<()>::internal_error("Failed to get system log")
        }
    }
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

    // Endpoint is only active if both private and public keys are configured.
    // Return 404 if keys are not set (endpoint doesn't exist).
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
