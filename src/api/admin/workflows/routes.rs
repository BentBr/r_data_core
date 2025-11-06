use actix_web::{delete, get, post, put, web, Responder};
use log::error;
use uuid::Uuid;
use std::str::FromStr;
use chrono::{Utc, DateTime};

use crate::api::admin::workflows::models::{
    CreateWorkflowRequest, CreateWorkflowResponse, UpdateWorkflowRequest, WorkflowDetail,
    WorkflowSummary, WorkflowRunLogDto, WorkflowRunSummary,
};
use crate::api::auth::auth_enum;
use crate::api::ApiResponse;
use crate::api::response::ValidationViolation;
use crate::api::ApiState;
use crate::api::query::PaginationQuery;
use crate::workflow::data::job_queue::apalis_redis::ApalisRedisQueue;
use crate::workflow::data::job_queue::JobQueue;
use crate::workflow::data::jobs::FetchAndStageJob;
use actix_multipart::Multipart;
use futures_util::StreamExt;

/// Register workflow routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_workflows)
        // Register static 'runs' routes BEFORE dynamic '/{uuid}' to avoid conflicts
        .service(list_all_workflow_runs)
        .service(cron_preview)
        .service(run_workflow_now_upload)
        .service(list_workflow_run_logs)
        .service(list_workflow_runs)
        // Dynamic UUID routes
        .service(get_workflow_details)
        .service(create_workflow)
        .service(update_workflow)
        .service(delete_workflow)
        .service(run_workflow_now);
}

/// Preview next run times for a cron expression
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflows/cron/preview",
    tag = "workflows",
    params(("expr" = String, Query, description = "Cron expression")),
    responses(
        (status = 200, description = "Preview next run times", body = [String]),
        (status = 422, description = "Invalid cron expression")
    ),
    security(("jwt" = []))
)]
#[get("/cron/preview")]
pub async fn cron_preview(
    query: web::Query<std::collections::HashMap<String, String>>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let expr = match query.get("expr") {
        Some(v) if !v.trim().is_empty() => v.clone(),
        _ => return ApiResponse::<()>::unprocessable_entity("Missing expr parameter"),
    };

    match cron::Schedule::from_str(&expr) {
        Ok(schedule) => {
            let next: Vec<String> = schedule
                .upcoming(Utc)
                .take(5)
                .map(|dt: DateTime<Utc>| dt.to_rfc3339())
                .collect();
            ApiResponse::ok(next)
        }
        Err(e) => ApiResponse::<()>::unprocessable_entity(&format!("Invalid cron: {}", e)),
    }
}
/// List all workflow runs across all workflows
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflows/runs",
    tag = "workflows",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Items per page (default: 20, max: 100)")
    ),
    responses((status = 200, description = "List all workflow runs (paginated)", body = [WorkflowRunSummary])),
    security(("jwt" = []))
)]
#[get("/runs")]
pub async fn list_all_workflow_runs(
    state: web::Data<ApiState>,
    query: web::Query<PaginationQuery>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    match state
        .workflow_service
        .list_all_runs_paginated(limit, offset)
        .await
    {
        Ok((items, total)) => {
            let summaries: Vec<WorkflowRunSummary> = items
                .into_iter()
                .map(|(uuid, status, queued_at, finished_at, processed, failed)| WorkflowRunSummary {
                    uuid,
                    status,
                    queued_at,
                    started_at: None,
                    finished_at,
                    processed_items: processed,
                    failed_items: failed,
                })
                .collect();
            ApiResponse::ok_paginated(summaries, total, page, per_page)
        }
        Err(e) => {
            error!(
                target: "workflows",
                "list_all_workflow_runs failed: {:#?}",
                e
            );
            ApiResponse::<()>::internal_error(&format!("Failed to list runs: {}", e))
        }
    }
}

/// List available workflows
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflows",
    tag = "workflows",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Items per page (default: 20, max: 100)"),
        ("limit" = Option<i64>, Query, description = "Alternative to per_page"),
        ("offset" = Option<i64>, Query, description = "Alternative to page-based")
    ),
    responses(
        (status = 200, description = "List workflows (paginated)", body = [WorkflowSummary])
    ),
    security(("jwt" = []))
)]
#[get("")]
pub async fn list_workflows(
    state: web::Data<ApiState>,
    _: auth_enum::RequiredAuth,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    match state.workflow_service.list_paginated(limit, offset).await {
        Ok((items, total)) => {
            let summaries: Vec<WorkflowSummary> = items
                .into_iter()
                .map(|workflow| WorkflowSummary {
                    uuid: workflow.uuid,
                    name: workflow.name,
                    kind: workflow.kind,
                    enabled: workflow.enabled,
                    schedule_cron: workflow.schedule_cron,
                })
                .collect();
            ApiResponse::ok_paginated(summaries, total, page, per_page)
        }
        Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to list workflows: {}", e)),
    }
}

/// Get details for one workflow by UUID
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflows/{uuid}",
    tag = "workflows",
    params(("uuid" = Uuid, Path, description = "Workflow UUID")),
    responses(
        (status = 200, description = "Workflow details", body = WorkflowDetail)
    ),
    security(
        ("jwt" = [])
    )
)]
#[get("/{uuid}")]
pub async fn get_workflow_details(
    state: web::Data<ApiState>,
    path: web::Path<Uuid>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let uuid = path.into_inner();
    match state.workflow_service.get(uuid).await {
        Ok(Some(workflow)) => {
            let detail = WorkflowDetail {
                uuid: workflow.uuid,
                name: workflow.name,
                description: workflow.description,
                kind: workflow.kind,
                enabled: workflow.enabled,
                schedule_cron: workflow.schedule_cron,
                config: workflow.config,
            };
            ApiResponse::ok(detail)
        }
        Ok(None) => ApiResponse::<()>::not_found("Workflow not found"),
        Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to get workflow: {}", e)),
    }
}

/// Create a new workflow
#[utoipa::path(
    post,
    path = "/admin/api/v1/workflows",
    tag = "workflows",
    request_body = CreateWorkflowRequest,
    responses(
        (status = 201, description = "Created", body = CreateWorkflowResponse),
        (status = 409, description = "Conflict - Workflow name already exists"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("")]
pub async fn create_workflow(
    state: web::Data<ApiState>,
    body: web::Json<CreateWorkflowRequest>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    // Validate cron format early to return Symfony-style 422
    if let Some(cron_str) = &body.schedule_cron {
        if let Err(e) = cron::Schedule::from_str(cron_str) {
            return ApiResponse::<()>::unprocessable_entity_with_violations(
                &format!("Invalid cron schedule: {}", e),
                vec![ValidationViolation {
                    field: "schedule_cron".to_string(),
                    message: "Invalid cron expression".to_string(),
                    code: Some("INVALID_CRON".to_string()),
                }],
            );
        }
    }

    let created = state.workflow_service.create(&body.0).await;

    match created {
        Ok(uuid) => ApiResponse::<CreateWorkflowResponse>::created(CreateWorkflowResponse { uuid }),
        Err(e) => {
            // Log the full error (captured by Actix logger middleware)
            error!("Failed to create workflow: {e}");

            // Map unique constraint violations to 409 Conflict
            // See the db error code of 23505
            if matches!(
                e.downcast_ref::<sqlx::Error>(),
                Some(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505")
            ) {
                return ApiResponse::<()>::conflict("Workflow name already exists");
            }

            ApiResponse::<()>::internal_error("Failed to create workflow")
        },
    }
}

/// Update a workflow by UUID
#[utoipa::path(
    put,
    path = "/admin/api/v1/workflows/{uuid}",
    tag = "workflows",
    params(("uuid" = Uuid, Path, description = "Workflow UUID")),
    request_body = UpdateWorkflowRequest,
    responses((status = 200, description = "Updated")),
    security(
        ("jwt" = [])
    )
)]
#[put("/{uuid}")]
pub async fn update_workflow(
    state: web::Data<ApiState>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateWorkflowRequest>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let uuid = path.into_inner();
    // Validate cron format early to return Symfony-style 422
    if let Some(cron_str) = &body.schedule_cron {
        if let Err(e) = cron::Schedule::from_str(cron_str) {
            return ApiResponse::<()>::unprocessable_entity_with_violations(
                &format!("Invalid cron schedule: {}", e),
                vec![ValidationViolation {
                    field: "schedule_cron".to_string(),
                    message: "Invalid cron expression".to_string(),
                    code: Some("INVALID_CRON".to_string()),
                }],
            );
        }
    }

    let res = state.workflow_service.update(uuid, &body.0).await;

    match res {
        Ok(_) => ApiResponse::<()>::message("Updated"),
        Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to update: {}", e)),
    }
}

/// Delete a workflow by UUID
#[utoipa::path(
    delete,
    path = "/admin/api/v1/workflows/{uuid}",
    tag = "workflows",
    params(("uuid" = Uuid, Path, description = "Workflow UUID")),
    responses((status = 200, description = "Deleted")),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{uuid}")]
pub async fn delete_workflow(
    state: web::Data<ApiState>,
    path: web::Path<Uuid>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let uuid = path.into_inner();
    let res = state.workflow_service.delete(uuid).await;
    match res {
        Ok(_) => ApiResponse::<()>::message("Deleted"),
        Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to delete: {}", e)),
    }
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
    state: web::Data<ApiState>,
    path: web::Path<Uuid>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let uuid = path.into_inner();
    match state.workflow_service.get(uuid).await {
        Ok(Some(_)) => {
            match state.workflow_service.enqueue_run(uuid).await {
                Ok(run_uuid) => {
                    let _ = ApalisRedisQueue::new()
                        .enqueue_fetch(FetchAndStageJob { workflow_id: uuid, trigger_id: Some(run_uuid) })
                        .await;
                    ApiResponse::<()>::message("Workflow run enqueued")
                }
                Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to enqueue run: {}", e)),
            }
        }
        Ok(None) => ApiResponse::<()>::not_found("Workflow"),
        Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to fetch workflow: {}", e)),
    }
}

/// Upload a file and stage raw items for a workflow run (Run Now)
#[utoipa::path(
    post,
    path = "/admin/api/v1/workflows/{uuid}/run/upload",
    tag = "workflows",
    params(("uuid" = Uuid, Path, description = "Workflow UUID")),
    request_body(
        content = inline(crate::api::admin::workflows::models::WorkflowRunUpload),
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
#[post("/{uuid}/run/upload")]
pub async fn run_workflow_now_upload(
    state: web::Data<ApiState>,
    path: web::Path<Uuid>,
    mut payload: Multipart,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let workflow_uuid = path.into_inner();
    // Validate workflow exists
    match state.workflow_service.get(workflow_uuid).await {
        Ok(Some(_)) => {}
        Ok(None) => return ApiResponse::<()>::not_found("Workflow"),
        Err(e) => return ApiResponse::<()>::internal_error(&format!("Failed to fetch workflow: {}", e)),
    }

    // Read multipart, find first 'file' part
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
    if file_bytes.is_empty() {
        return ApiResponse::<()>::bad_request("Missing file");
    }

    match state.workflow_service.run_now_upload_csv(workflow_uuid, &file_bytes).await {
        Ok((run_uuid, staged)) => {
            ApiResponse::<serde_json::Value>::ok(serde_json::json!({
                "run_uuid": run_uuid,
                "staged_items": staged
            }))
        }
        Err(e) => {
            error!(target: "workflows", "run_workflow_now_upload failed: {:#?}", e);
            // Treat CSV parse issues as 422 to surface validation to the UI
            let msg = e.to_string();
            if msg.contains("CSV") || msg.contains("Failed to read") {
                ApiResponse::<()>::unprocessable_entity(&msg)
            } else {
                ApiResponse::<()>::internal_error(&format!("Failed to process upload: {}", msg))
            }
        },
    }
}

/// List workflow runs (history)
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflows/{uuid}/runs",
    tag = "workflows",
    params(
        ("uuid" = Uuid, Path, description = "Workflow UUID"),
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Items per page (default: 20, max: 100)")
    ),
    responses((status = 200, description = "List workflow runs (paginated)", body = [WorkflowRunSummary])),
    security(("jwt" = []))
)]
#[get("/{uuid}/runs")]
pub async fn list_workflow_runs(
    state: web::Data<ApiState>,
    path: web::Path<Uuid>,
    query: web::Query<PaginationQuery>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let workflow_uuid = path.into_inner();
    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    match state
        .workflow_service
        .list_runs_paginated(workflow_uuid, limit, offset)
        .await
    {
        Ok((items, total)) => {
            let summaries: Vec<WorkflowRunSummary> = items
                .into_iter()
                .map(|(uuid, status, queued_at, finished_at, processed, failed)| WorkflowRunSummary {
                    uuid,
                    status,
                    queued_at,
                    started_at: None,
                    finished_at,
                    processed_items: processed,
                    failed_items: failed,
                })
                .collect();
            ApiResponse::ok_paginated(summaries, total, page, per_page)
        }
        Err(e) => {
            error!(target: "workflows", "list_workflow_runs failed: {:#?}", e);
            ApiResponse::<()>::internal_error(&format!("Failed to list runs: {}", e))
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
    state: web::Data<ApiState>,
    path: web::Path<Uuid>,
    query: web::Query<PaginationQuery>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let run_uuid = path.into_inner();
    let (limit, offset) = query.to_limit_offset(50, 200);
    let page = query.get_page(1);
    let per_page = query.get_per_page(50, 200);

    // Return 404 if run does not exist
    match state.workflow_service.run_exists(run_uuid).await {
        Ok(false) => return ApiResponse::<()>::not_found("Workflow run not found"),
        Err(e) => return ApiResponse::<()>::internal_error(&format!("Failed to check run: {}", e)),
        Ok(true) => {}
    }

    match state
        .workflow_service
        .list_run_logs_paginated(run_uuid, limit, offset)
        .await
    {
        Ok((items, total)) => {
            let logs: Vec<WorkflowRunLogDto> = items
                .into_iter()
                .map(|(uuid, ts, level, message, meta)| WorkflowRunLogDto { uuid, ts, level, message, meta })
                .collect();
            ApiResponse::ok_paginated(logs, total, page, per_page)
        }
        Err(e) => {
            error!(target: "workflows", "list_workflow_run_logs failed: {:#?}", e);
            ApiResponse::<()>::internal_error(&format!("Failed to list run logs: {}", e))
        }
    }
}
