#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_multipart::Multipart;
use actix_web::{get, post, web, Responder};
use futures_util::StreamExt;
use log::{error, info};
use serde_json::json;
use uuid::Uuid;

use crate::admin::workflows::models::WorkflowRunLogDto;
use crate::admin::workflows::routes::utils::handle_workflow_error;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::query::PaginationQuery;
use crate::response::ApiResponse;
use r_data_core_core::error::Error;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;

/// Extract file from multipart payload
/// This function processes the multipart stream and returns the file bytes.
/// It doesn't need to be Send since it's the only async operation on the payload.
#[allow(clippy::future_not_send)] // Multipart is not Send, but this is the only async operation on it
async fn extract_file_from_multipart(mut payload: Multipart) -> Result<Vec<u8>, String> {
    let mut file_bytes: Vec<u8> = Vec::new();
    while let Some(Ok(mut field)) = payload.next().await {
        let name = field.name().to_string();
        if name != "file" {
            // drain non-file fields
            while let Some(Ok(_)) = field.next().await {}
            continue;
        }
        while let Some(Ok(chunk)) = field.next().await {
            file_bytes.extend_from_slice(&chunk);
        }
        break;
    }
    Ok(file_bytes)
}

/// Trigger a workflow by UUID immediately
#[utoipa::path(
    post,
    path = "/admin/api/v1/workflows/{uuid}/run",
    tag = "workflows",
    params(("uuid" = Uuid, Path, description = "Workflow UUID")),
    responses(
        (status = 202, description = "Enqueued"),
        (status = 404, description = "Workflow not found")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/{uuid}/run")]
pub async fn run_workflow_now(
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Execute,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to execute workflows");
    }

    let uuid = path.into_inner();
    match state.workflow_service().get(uuid).await {
        Ok(Some(_)) => match state.workflow_service().enqueue_run(uuid).await {
            Ok(run_uuid) => {
                match state
                    .queue()
                    .enqueue_fetch(FetchAndStageJob {
                        workflow_id: uuid,
                        trigger_id: Some(run_uuid),
                    })
                    .await
                {
                    Ok(()) => {
                        info!(
                            "Successfully enqueued fetch job for workflow {uuid} (run: {run_uuid})"
                        );
                        ApiResponse::<serde_json::Value>::ok(json!({
                            "status": "queued",
                            "run_uuid": run_uuid,
                            "message": "Workflow run enqueued"
                        }))
                    }
                    Err(e) => {
                        error!(
                            "Failed to enqueue fetch job for workflow {uuid} (run: {run_uuid}): {e}"
                        );
                        ApiResponse::<()>::internal_error(&format!(
                            "Failed to enqueue job to Redis: {e}"
                        ))
                    }
                }
            }
            Err(e) => handle_workflow_error(e),
        },
        Ok(None) => ApiResponse::<()>::not_found("Workflow"),
        Err(e) => handle_workflow_error(e),
    }
}

/// Upload a file and stage raw items for a workflow run (Run Now)
#[utoipa::path(
    post,
    path = "/admin/api/v1/workflows/{uuid}/run/upload",
    tag = "workflows",
    params(("uuid" = Uuid, Path, description = "Workflow UUID")),
    request_body(
        content = inline(crate::admin::workflows::models::WorkflowRunUpload),
        content_type = "multipart/form-data",
        description = "CSV file uploaded as multipart/form-data with field name 'file'"
    ),
    responses(
        (status = 200, description = "Uploaded and staged", body = inline(serde_json::Value)),
        (status = 404, description = "Workflow not found"),
        (status = 400, description = "Bad request")
    ),
    security(("jwt" = []))
)]
#[allow(clippy::future_not_send)] // Calls extract_file_from_multipart which is not Send
#[post("/{uuid}/run/upload")]
pub async fn run_workflow_now_upload(
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    payload: Multipart,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Execute,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to execute workflows");
    }

    let workflow_uuid = path.into_inner();
    // Validate workflow exists
    match state.workflow_service().get(workflow_uuid).await {
        Ok(Some(_)) => {}
        Ok(None) => return ApiResponse::<()>::not_found("Workflow"),
        Err(e) => return handle_workflow_error(e),
    }

    // Extract file from multipart before any Send-requiring operations
    let file_bytes = extract_file_from_multipart(payload)
        .await
        .unwrap_or_default();
    if file_bytes.is_empty() {
        return ApiResponse::<()>::bad_request("Missing file");
    }

    match state
        .workflow_service()
        .run_now_upload_csv(workflow_uuid, &file_bytes)
        .await
    {
        Ok((run_uuid, staged)) => {
            // Enqueue job to process the staged items (worker will skip fetching since items are already staged)
            match state
                .queue()
                .enqueue_fetch(FetchAndStageJob {
                    workflow_id: workflow_uuid,
                    trigger_id: Some(run_uuid),
                })
                .await
            {
                Ok(()) => {
                    info!("Successfully enqueued fetch job for uploaded workflow {workflow_uuid} (run: {run_uuid}, staged: {staged})");
                    ApiResponse::<serde_json::Value>::ok(serde_json::json!({
                        "run_uuid": run_uuid,
                        "staged_items": staged
                    }))
                }
                Err(e) => {
                    error!(
                        "Failed to enqueue fetch job for uploaded workflow {workflow_uuid} (run: {run_uuid}): {e}"
                    );
                    // Still return success for the upload, but log the enqueue failure
                    ApiResponse::<serde_json::Value>::ok(serde_json::json!({
                        "run_uuid": run_uuid,
                        "staged_items": staged,
                        "warning": "Upload succeeded but job enqueue failed - items may not be processed automatically"
                    }))
                }
            }
        }
        Err(Error::Validation(msg)) => ApiResponse::<()>::unprocessable_entity(&msg),
        Err(e) => {
            error!(target: "workflows", "run_workflow_now_upload failed: {e:#?}");
            handle_workflow_error(e)
        }
    }
}

/// List logs for a workflow run
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflow-runs/{run_uuid}/logs",
    tag = "workflows",
    params(
        ("run_uuid" = Uuid, Path, description = "Workflow run UUID"),
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Items per page (default: 50, max: 200)")
    ),
    responses((status = 200, description = "List workflow run logs (paginated)", body = [WorkflowRunLogDto])),
    security(("jwt" = []))
)]
#[get("/runs/{run_uuid}/logs")]
pub async fn list_workflow_run_logs(
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    query: web::Query<PaginationQuery>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view workflow run logs");
    }

    let run_uuid = path.into_inner();
    let (limit, offset) = query.to_limit_offset(50, 200);
    let page = query.get_page(1);
    let per_page = query.get_per_page(50, 200);

    // Return 404 if run does not exist
    match state.workflow_service().run_exists(run_uuid).await {
        Ok(false) => return ApiResponse::<()>::not_found("Workflow run not found"),
        Err(e) => return handle_workflow_error(e),
        Ok(true) => {}
    }

    match state
        .workflow_service()
        .list_run_logs_paginated(run_uuid, limit, offset)
        .await
    {
        Ok((items, total)) => {
            let logs: Vec<WorkflowRunLogDto> = items
                .into_iter()
                .map(|(uuid, ts, level, message, meta)| WorkflowRunLogDto {
                    uuid,
                    ts,
                    level,
                    message,
                    meta,
                })
                .collect();
            ApiResponse::ok_paginated(logs, total, page, per_page)
        }
        Err(e) => {
            error!(target: "workflows", "list_workflow_run_logs failed: {e:#?}");
            handle_workflow_error(e)
        }
    }
}
