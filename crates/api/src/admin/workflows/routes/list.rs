#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, web, Responder};
use log::error;
use std::sync::Arc;
use uuid::Uuid;

use super::utils::check_has_api_endpoint;
use crate::admin::query_helpers::to_list_query_params;
use crate::admin::workflows::models::{WorkflowRunSummary, WorkflowSummary};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::query::{PaginationQuery, StandardQuery};
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_services::query_validation::FieldValidator;

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
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to list workflow runs");
    }

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
                "list_all_workflow_runs failed: {e:#?}"
            );
            ApiResponse::<()>::internal_error(&format!("Failed to list runs: {e}"))
        }
    }
}

/// List available workflows with pagination and sorting
#[utoipa::path(
    get,
    path = "/admin/api/v1/workflows",
    tag = "workflows",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-based, default: 1)"),
        ("per_page" = Option<i64>, Query, description = "Items per page (default: 20, max: 100, or -1 for unlimited)"),
        ("limit" = Option<i64>, Query, description = "Alternative to per_page"),
        ("offset" = Option<i64>, Query, description = "Alternative to page-based"),
        ("sort_by" = Option<String>, Query, description = "Field to sort by (e.g., name, enabled, created_at)"),
        ("sort_order" = Option<String>, Query, description = "Sort order: 'asc' or 'desc' (default: 'asc')")
    ),
    responses(
        (status = 200, description = "List workflows (paginated)", body = [WorkflowSummary]),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 500, description = "Internal server error")
    ),
    security(("jwt" = []))
)]
#[get("")]
pub async fn list_workflows(
    state: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
    query: web::Query<StandardQuery>,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to list workflows");
    }

    // Create field validator
    let pool = Arc::new(state.db_pool().clone());
    let field_validator = Arc::new(FieldValidator::new(pool));

    // Convert StandardQuery to ListQueryParams and use service method that handles all validation
    let params = to_list_query_params(&query);
    match state
        .workflow_service()
        .list_paginated_with_query(&params, &field_validator)
        .await
    {
        Ok(((items, total), validated)) => {
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
            ApiResponse::ok_paginated(summaries, total, validated.page, validated.per_page)
        }
        Err(e) => {
            error!("Failed to list workflows: {e}");
            let err_msg = e.to_string();
            if err_msg.contains("validation") {
                ApiResponse::<()>::bad_request(&err_msg)
            } else {
                ApiResponse::<()>::internal_error(&format!("Failed to list workflows: {e}"))
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
        return ApiResponse::<()>::forbidden("Insufficient permissions to list workflow runs");
    }

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
            error!(target: "workflows", "list_workflow_runs failed: {e:#?}");
            ApiResponse::<()>::internal_error(&format!("Failed to list runs: {e}"))
        }
    }
}
