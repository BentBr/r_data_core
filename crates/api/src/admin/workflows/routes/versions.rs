#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, web, Responder};
use log::error;
use uuid::Uuid;

use crate::admin::workflows::models::{WorkflowVersionMeta, WorkflowVersionPayload};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_persistence::WorkflowVersioningRepository;

/// List versions of a workflow
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflows/{uuid}/versions",
    tag = "workflows",
    params(
        ("uuid" = Uuid, Path, description = "Workflow UUID")
    ),
    responses(
        (status = 200, description = "List of versions", body = Vec<WorkflowVersionMeta>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workflow not found"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/{uuid}/versions")]
pub async fn list_workflow_versions(
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view workflow versions");
    }

    let workflow_uuid = path.into_inner();
    let versioning_repo = WorkflowVersioningRepository::new(state.db_pool().clone());

    // Get historical versions
    let rows = match versioning_repo.list_workflow_versions(workflow_uuid).await {
        Ok(rows) => rows,
        Err(e) => {
            error!("Failed to list workflow versions: {e}");
            return ApiResponse::<()>::internal_error("Failed to list versions");
        }
    };

    // Get current workflow metadata
    let current_metadata = match versioning_repo
        .get_current_workflow_metadata(workflow_uuid)
        .await
    {
        Ok(metadata) => metadata,
        Err(e) => {
            error!("Failed to get current workflow metadata: {e}");
            return ApiResponse::<()>::internal_error("Failed to get current metadata");
        }
    };

    let mut out: Vec<WorkflowVersionMeta> = Vec::new();

    // Add current version if it exists and is not in the versions table
    if let Some((version, updated_at, updated_by, updated_by_name)) = current_metadata {
        let is_in_versions = rows.iter().any(|r| r.version_number == version);
        if !is_in_versions {
            out.push(WorkflowVersionMeta {
                version_number: version,
                created_at: updated_at,
                created_by: updated_by,
                created_by_name: updated_by_name,
            });
        }
    }

    // Add all historical versions
    for r in rows {
        out.push(WorkflowVersionMeta {
            version_number: r.version_number,
            created_at: r.created_at,
            created_by: r.created_by,
            created_by_name: r.created_by_name,
        });
    }

    // Sort by version number descending (newest first)
    out.sort_by(|a, b| b.version_number.cmp(&a.version_number));

    ApiResponse::ok(out)
}

/// Get a specific version snapshot of a workflow
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflows/{uuid}/versions/{version_number}",
    tag = "workflows",
    params(
        ("uuid" = Uuid, Path, description = "Workflow UUID"),
        ("version_number" = i32, Path, description = "Version number")
    ),
    responses(
        (status = 200, description = "Version snapshot", body = WorkflowVersionPayload),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Version not found"),
        (status = 500, description = "Server error")
    ),
    security(("jwt" = []))
)]
#[get("/{uuid}/versions/{version_number}")]
pub async fn get_workflow_version(
    state: web::Data<ApiStateWrapper>,
    path: web::Path<(Uuid, i32)>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view workflow versions");
    }

    let (workflow_uuid, version_number) = path.into_inner();
    let versioning_repo = WorkflowVersioningRepository::new(state.db_pool().clone());

    // First try to get from versions table
    match versioning_repo
        .get_workflow_version(workflow_uuid, version_number)
        .await
    {
        Ok(Some(row)) => {
            let payload = WorkflowVersionPayload {
                version_number: row.version_number,
                created_at: row.created_at,
                created_by: row.created_by,
                data: row.data,
            };
            return ApiResponse::ok(payload);
        }
        Ok(None) => {
            // Not in versions table, check if it's the current version
            let current_metadata = versioning_repo
                .get_current_workflow_metadata(workflow_uuid)
                .await
                .ok()
                .flatten();

            if let Some((current_version, updated_at, updated_by, _updated_by_name)) =
                current_metadata
            {
                if current_version == version_number {
                    // This is the current version, fetch from workflows table
                    if let Ok(Some(workflow)) = state.workflow_service().get(workflow_uuid).await {
                        let current_json = serde_json::to_value(&workflow)
                            .unwrap_or_else(|_| serde_json::json!({}));
                        let payload = WorkflowVersionPayload {
                            version_number,
                            created_at: updated_at,
                            created_by: updated_by,
                            data: current_json,
                        };
                        return ApiResponse::ok(payload);
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to get workflow version: {e}");
            return ApiResponse::<()>::internal_error("Failed to get version");
        }
    }

    ApiResponse::<()>::not_found("Version not found")
}
