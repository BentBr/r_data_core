use r_data_core_core::utils;
use actix_web::{delete, get, post, put, web, Responder};
use log::{error, info};
use serde_json::Value;
use uuid::Uuid;

use r_data_core_api::admin::workflows::models::{
    CreateWorkflowRequest, CreateWorkflowResponse, UpdateWorkflowRequest, WorkflowDetail,
    WorkflowRunLogDto, WorkflowRunSummary, WorkflowSummary, WorkflowVersionMeta,
    WorkflowVersionPayload,
};
use crate::api::auth::auth_enum;
use r_data_core_api::query::PaginationQuery;
use r_data_core_api::response::ValidationViolation;
use r_data_core_api::response::ApiResponse;
use crate::api::ApiState;
use crate::workflow::data::job_queue::JobQueue;
use crate::workflow::data::jobs::FetchAndStageJob;
use r_data_core_persistence::WorkflowVersioningRepository;
use actix_multipart::Multipart;
use futures_util::StreamExt;

/// Check if a workflow config has from.api source type (accepts POST, cron disabled)
fn check_has_api_endpoint(config: &Value) -> bool {
    if let Some(steps) = config.get("steps").and_then(|v| v.as_array()) {
        for step in steps {
            if let Some(from) = step.get("from") {
                // Check for from.format.source.source_type === "api" without endpoint field
                if let Some(source) = from
                    .get("source")
                    .or_else(|| from.get("format").and_then(|f| f.get("source")))
                {
                    if let Some(source_type) = source.get("source_type").and_then(|v| v.as_str()) {
                        if source_type == "api" {
                            // from.api without endpoint field = accepts POST
                            if let Some(config_obj) =
                                source.get("config").and_then(|v| v.as_object())
                            {
                                if !config_obj.contains_key("endpoint") {
                                    return true;
                                }
                            } else {
                                // No config object or empty config = accepts POST
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

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
        .service(run_workflow_now)
        .service(list_workflow_versions)
        .service(get_workflow_version);
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

    match utils::preview_next(&expr, 5) {
        Ok(next) => ApiResponse::ok(next),
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
                .map(
                    |(uuid, status, queued_at, finished_at, processed, failed)| {
                        WorkflowRunSummary {
                            uuid,
                            status,
                            queued_at,
                            started_at: None,
                            finished_at,
                            processed_items: processed,
                            failed_items: failed,
                        }
                    },
                )
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
                .map(|workflow| {
                    // Check if workflow has from.api source type (accepts POST, cron disabled)
                    let has_api_endpoint = check_has_api_endpoint(&workflow.config);
                    WorkflowSummary {
                        uuid: workflow.uuid,
                        name: workflow.name,
                        kind: format!("{:?}", workflow.kind),
                        enabled: workflow.enabled,
                        schedule_cron: workflow.schedule_cron,
                        has_api_endpoint,
                        versioning_disabled: workflow.versioning_disabled,
                    }
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
                kind: format!("{:?}", workflow.kind),
                enabled: workflow.enabled,
                schedule_cron: workflow.schedule_cron,
                config: workflow.config,
                versioning_disabled: workflow.versioning_disabled,
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
    auth: auth_enum::RequiredAuth,
) -> impl Responder {
    // Validate cron format early to return Symfony-style 422
    if let Some(cron_str) = &body.schedule_cron {
        if let Err(e) = utils::validate_cron(cron_str) {
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

    // Determine creator from required auth (JWT)
    let created_by = match auth.user_uuid() {
        Some(u) => u,
        None => return ApiResponse::<()>::internal_error("No authentication claims found"),
    };

    let created = state.workflow_service.create(&body.0, created_by).await;

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
        }
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
    auth: auth_enum::RequiredAuth,
) -> impl Responder {
    let uuid = path.into_inner();
    let updated_by = match auth.user_uuid() {
        Some(u) => u,
        None => return ApiResponse::<()>::internal_error("No authentication claims found"),
    };
    // Validate cron format early to return Symfony-style 422
    if let Some(cron_str) = &body.schedule_cron {
        if let Err(e) = utils::validate_cron(cron_str) {
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

    let res = state
        .workflow_service
        .update(uuid, &body.0, updated_by)
        .await;

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
        Ok(Some(_)) => match state.workflow_service.enqueue_run(uuid).await {
            Ok(run_uuid) => {
                match state
                    .queue
                    .enqueue_fetch(FetchAndStageJob {
                        workflow_id: uuid,
                        trigger_id: Some(run_uuid),
                    })
                    .await
                {
                    Ok(_) => {
                        info!(
                            "Successfully enqueued fetch job for workflow {} (run: {})",
                            uuid, run_uuid
                        );
                        ApiResponse::<()>::message("Workflow run enqueued")
                    }
                    Err(e) => {
                        error!(
                            "Failed to enqueue fetch job for workflow {} (run: {}): {}",
                            uuid, run_uuid, e
                        );
                        ApiResponse::<()>::internal_error(&format!(
                            "Failed to enqueue job to Redis: {}",
                            e
                        ))
                    }
                }
            }
            Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to enqueue run: {}", e)),
        },
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
        content = inline(r_data_core_api::admin::workflows::models::WorkflowRunUpload),
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
        Err(e) => {
            return ApiResponse::<()>::internal_error(&format!("Failed to fetch workflow: {}", e))
        }
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

    match state
        .workflow_service
        .run_now_upload_csv(workflow_uuid, &file_bytes)
        .await
    {
        Ok((run_uuid, staged)) => {
            // Enqueue job to process the staged items (worker will skip fetching since items are already staged)
            match state
                .queue
                .enqueue_fetch(FetchAndStageJob {
                    workflow_id: workflow_uuid,
                    trigger_id: Some(run_uuid),
                })
                .await
            {
                Ok(_) => {
                    info!("Successfully enqueued fetch job for uploaded workflow {} (run: {}, staged: {})", workflow_uuid, run_uuid, staged);
                    ApiResponse::<serde_json::Value>::ok(serde_json::json!({
                        "run_uuid": run_uuid,
                        "staged_items": staged
                    }))
                }
                Err(e) => {
                    error!(
                        "Failed to enqueue fetch job for uploaded workflow {} (run: {}): {}",
                        workflow_uuid, run_uuid, e
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
        Err(e) => {
            error!(target: "workflows", "run_workflow_now_upload failed: {:#?}", e);
            // Treat CSV parse issues as 422 to surface validation to the UI
            let msg = e.to_string();
            if msg.contains("CSV") || msg.contains("Failed to read") {
                ApiResponse::<()>::unprocessable_entity(&msg)
            } else {
                ApiResponse::<()>::internal_error(&format!("Failed to process upload: {}", msg))
            }
        }
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
                .map(
                    |(uuid, status, queued_at, finished_at, processed, failed)| {
                        WorkflowRunSummary {
                            uuid,
                            status,
                            queued_at,
                            started_at: None,
                            finished_at,
                            processed_items: processed,
                            failed_items: failed,
                        }
                    },
                )
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
            error!(target: "workflows", "list_workflow_run_logs failed: {:#?}", e);
            ApiResponse::<()>::internal_error(&format!("Failed to list run logs: {}", e))
        }
    }
}


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
    state: web::Data<ApiState>,
    path: web::Path<Uuid>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let workflow_uuid = path.into_inner();
    let versioning_repo = WorkflowVersioningRepository::new(state.db_pool.clone());

    // Get historical versions
    let rows = match versioning_repo.list_workflow_versions(workflow_uuid).await {
        Ok(rows) => rows,
        Err(e) => {
            error!("Failed to list workflow versions: {}", e);
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
            error!("Failed to get current workflow metadata: {}", e);
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
    state: web::Data<ApiState>,
    path: web::Path<(Uuid, i32)>,
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let (workflow_uuid, version_number) = path.into_inner();
    let versioning_repo = WorkflowVersioningRepository::new(state.db_pool.clone());

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
                    if let Ok(Some(workflow)) = state.workflow_service.get(workflow_uuid).await {
                        let current_json =
                            serde_json::to_value(&workflow).unwrap_or(serde_json::json!({}));
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
            error!("Failed to get workflow version: {}", e);
            return ApiResponse::<()>::internal_error("Failed to get version");
        }
    }

    ApiResponse::<()>::not_found("Version not found")
}
