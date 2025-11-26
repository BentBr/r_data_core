#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, web, Responder};
use log::error;
use uuid::Uuid;

use crate::admin::workflows::models::{WorkflowRunSummary, WorkflowSummary};
use crate::auth::auth_enum::RequiredAuth;
use crate::query::PaginationQuery;
use crate::response::ApiResponse;
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use super::utils::check_has_api_endpoint;

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
    state: web::Data<ApiStateWrapper>,
    query: web::Query<PaginationQuery>,
    _: RequiredAuth,
) -> impl Responder {
    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    match state
        .workflow_service()
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
    state: web::Data<ApiStateWrapper>,
    _: RequiredAuth,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    match state.workflow_service().list_paginated(limit, offset).await {
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
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    query: web::Query<PaginationQuery>,
    _: RequiredAuth,
) -> impl Responder {
    let workflow_uuid = path.into_inner();
    let (limit, offset) = query.to_limit_offset(20, 100);
    let page = query.get_page(1);
    let per_page = query.get_per_page(20, 100);

    match state
        .workflow_service()
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

