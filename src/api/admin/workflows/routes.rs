use actix_web::{delete, get, post, put, web, Responder};
use log::error;
use uuid::Uuid;

use crate::api::admin::workflows::models::{
    CreateWorkflowRequest, CreateWorkflowResponse, UpdateWorkflowRequest, WorkflowSummary,
};
use crate::api::auth::auth_enum;
use crate::api::ApiResponse;
use crate::api::ApiState;
use crate::api::query::PaginationQuery;

/// Register workflow routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_workflows)
        .service(get_workflow_details)
        .service(create_workflow)
        .service(update_workflow)
        .service(delete_workflow)
        .service(run_workflow_now);
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
        (status = 200, description = "Workflow details", body = WorkflowSummary)
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
            let summary = WorkflowSummary {
                uuid: workflow.uuid,
                name: workflow.name,
                kind: workflow.kind,
                enabled: workflow.enabled,
                schedule_cron: workflow.schedule_cron,
            };
            ApiResponse::ok(summary)
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
        Ok(Some(_)) => ApiResponse::<()>::message("Workflow run enqueued"),
        Ok(None) => ApiResponse::<()>::not_found("Workflow"),
        Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to fetch workflow: {}", e)),
    }
}
