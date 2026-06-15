#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::admin::system::models::{
    EntityVersioningSettingsDto, OutboxSettingsDto, UpdateOutboxSettingsBody, UpdateSettingsBody,
    UpdateWorkflowRunLogSettingsBody, WorkflowRunLogSettingsDto,
};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::ApiResponse;
use actix_web::{get, put, web, Responder};
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_services::SettingsService;

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
        Ok(()) => {
            if let Some(log_svc) = data.system_log_service() {
                log_svc
                    .log_entity_updated(
                        Some(updated_by),
                        r_data_core_core::system_log::SystemLogResourceType::SystemSettings,
                        updated_by, // no specific resource UUID, use actor
                        "Entity versioning settings updated",
                        Some(serde_json::json!({
                            "setting": "entity_versioning",
                            "enabled": current.enabled,
                            "max_versions": current.max_versions,
                            "max_age_days": current.max_age_days,
                        })),
                    )
                    .await;
            }
            ApiResponse::ok(EntityVersioningSettingsDto::from(current))
        }
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
        Ok(()) => {
            if let Some(log_svc) = data.system_log_service() {
                log_svc
                    .log_entity_updated(
                        Some(updated_by),
                        r_data_core_core::system_log::SystemLogResourceType::SystemSettings,
                        updated_by,
                        "Workflow run log settings updated",
                        Some(serde_json::json!({
                            "setting": "workflow_run_logs",
                            "enabled": current.enabled,
                            "max_runs": current.max_runs,
                            "max_age_days": current.max_age_days,
                        })),
                    )
                    .await;
            }
            ApiResponse::ok(WorkflowRunLogSettingsDto::from(current))
        }
        Err(e) => {
            log::error!("Failed to update workflow run log settings: {e}");
            ApiResponse::<()>::internal_error("Failed to update settings")
        }
    }
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/system/settings/outbox",
    tag = "system",
    responses(
        (status = 200, description = "Get outbox settings", body = OutboxSettingsDto),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/settings/outbox")]
pub async fn get_outbox_settings(
    data: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
) -> impl Responder {
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view system settings");
    }

    let service = SettingsService::new(data.db_pool().clone(), data.cache_manager().clone());
    match service.get_outbox_settings().await {
        Ok(settings) => ApiResponse::ok(OutboxSettingsDto::from(settings)),
        Err(e) => {
            log::error!("Failed to load outbox settings: {e}");
            ApiResponse::<()>::internal_error("Failed to load settings")
        }
    }
}

#[utoipa::path(
    put,
    path = "/admin/api/v1/system/settings/outbox",
    tag = "system",
    request_body = UpdateOutboxSettingsBody,
    responses(
        (status = 200, description = "Updated outbox settings", body = OutboxSettingsDto),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[put("/settings/outbox")]
pub async fn update_outbox_settings(
    data: web::Data<ApiStateWrapper>,
    body: web::Json<UpdateOutboxSettingsBody>,
    auth: RequiredAuth,
) -> impl Responder {
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::System,
        &PermissionType::Update,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to update system settings");
    }

    let service = SettingsService::new(data.db_pool().clone(), data.cache_manager().clone());
    let mut current = match service.get_outbox_settings().await {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to read current outbox settings: {e}");
            return ApiResponse::<()>::internal_error("Failed to read current settings");
        }
    };

    if let Some(v) = body.fetch_enabled {
        current.fetch_enabled = v;
    }
    if let Some(v) = body.push_enabled {
        current.push_enabled = v;
    }

    let Some(updated_by) = auth.user_uuid() else {
        return ApiResponse::<()>::internal_error("No authentication claims found for update");
    };

    match service.update_outbox_settings(&current, updated_by).await {
        Ok(()) => {
            if let Some(log_svc) = data.system_log_service() {
                log_svc
                    .log_entity_updated(
                        Some(updated_by),
                        r_data_core_core::system_log::SystemLogResourceType::SystemSettings,
                        updated_by,
                        "Outbox settings updated",
                        Some(serde_json::json!({
                            "setting": "outbox",
                            "fetch_enabled": current.fetch_enabled,
                            "push_enabled": current.push_enabled,
                        })),
                    )
                    .await;
            }
            ApiResponse::ok(OutboxSettingsDto::from(current))
        }
        Err(e) => {
            log::error!("Failed to update outbox settings: {e}");
            ApiResponse::<()>::internal_error("Failed to update settings")
        }
    }
}
