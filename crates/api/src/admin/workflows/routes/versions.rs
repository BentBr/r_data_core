#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, post, web, Responder};
use log::error;
use uuid::Uuid;

use crate::admin::workflows::models::{
    UpdateWorkflowRequest, WorkflowVersionMeta, WorkflowVersionPayload,
};
use crate::admin::workflows::routes::utils::handle_workflow_error;
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
    out.sort_by_key(|b| std::cmp::Reverse(b.version_number));

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

    // First, try to get from the versions table
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

/// Pull the DSL program out of a version snapshot.
///
/// Snapshots come in two shapes, because the two read paths build them
/// differently. A row from `workflow_versions` is `row_to_json` of the
/// `workflows` table, which has no `config` column at all — the program lives
/// in `consumer_config` or `provider_config` according to `kind`. The current
/// version instead comes from a serialized `Workflow`, which does have
/// `config`. Both are handled; neither is guessed at.
fn program_from_snapshot(data: &serde_json::Value) -> Option<&serde_json::Value> {
    if let Some(config) = data.get("config").filter(|v| !v.is_null()) {
        return Some(config);
    }
    let column = match data.get("kind").and_then(serde_json::Value::as_str)? {
        "consumer" => "consumer_config",
        "provider" => "provider_config",
        _ => return None,
    };
    data.get(column).filter(|v| !v.is_null())
}

/// Restore a previous version of a workflow.
///
/// Appends a **new** version carrying the old program. History is never
/// rewound: the change being rolled back stays in the record, and the restore
/// is itself an auditable event.
///
/// Only the program is restored. Taking `name`, `enabled` or `schedule_cron`
/// from the old row too would silently re-enable a workflow someone had
/// deliberately disabled, or rename it out from under a schedule.
#[utoipa::path(
    post,
    path = "/admin/api/v1/workflows/{uuid}/versions/{version_number}/restore",
    tag = "workflows",
    params(
        ("uuid" = Uuid, Path, description = "Workflow UUID"),
        ("version_number" = i32, Path, description = "Version to restore")
    ),
    responses(
        (status = 200, description = "Restored; a new version was appended"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Workflow or version not found"),
        (status = 422, description = "The stored version is not a valid DSL program")
    ),
    security(("jwt" = []))
)]
#[post("/{uuid}/versions/{version_number}/restore")]
pub async fn restore_workflow_version(
    state: web::Data<ApiStateWrapper>,
    path: web::Path<(Uuid, i32)>,
    auth: RequiredAuth,
) -> impl Responder {
    // Restore mutates the workflow, so it is gated like an update rather than
    // like the read-only version endpoints beside it.
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Update,
        None,
    ) {
        return ApiResponse::<()>::forbidden(
            "Insufficient permissions to restore workflow versions",
        );
    }

    let (workflow_uuid, version_number) = path.into_inner();
    let Some(updated_by) = auth.user_uuid() else {
        return ApiResponse::<()>::internal_error("No authentication claims found");
    };

    let versioning_repo = WorkflowVersioningRepository::new(state.db_pool().clone());
    let snapshot = match versioning_repo
        .get_workflow_version(workflow_uuid, version_number)
        .await
    {
        Ok(Some(row)) => row.data,
        Ok(None) => {
            return ApiResponse::<()>::not_found("Version not found");
        }
        Err(e) => {
            error!("Failed to read workflow version for restore: {e}");
            return ApiResponse::<()>::internal_error("Failed to read version");
        }
    };

    let Some(program) = program_from_snapshot(&snapshot) else {
        return ApiResponse::<()>::unprocessable_entity(&format!(
            "Version {version_number} contains no workflow configuration to restore"
        ));
    };

    // The current workflow supplies everything except the program, so a
    // restore cannot resurrect stale metadata.
    let current = match state.workflow_service().get(workflow_uuid).await {
        Ok(Some(workflow)) => workflow,
        Ok(None) => return ApiResponse::<()>::not_found("Workflow not found"),
        Err(e) => {
            error!("Failed to load workflow for restore: {e}");
            return ApiResponse::<()>::internal_error("Failed to load workflow");
        }
    };

    let request = UpdateWorkflowRequest {
        name: current.name.clone(),
        description: current.description.clone(),
        kind: current.kind.to_string(),
        enabled: current.enabled,
        schedule_cron: current.schedule_cron.clone(),
        config: program.clone(),
        versioning_disabled: current.versioning_disabled,
    };

    // `update` re-validates the program. A version stored before a DSL change
    // may no longer be runnable, and silently persisting one would turn a
    // rollback into an outage.
    match state
        .workflow_service()
        .update(workflow_uuid, &request, updated_by)
        .await
    {
        Ok(()) => ApiResponse::<()>::message("Restored"),
        Err(e) => handle_workflow_error(e),
    }
}
