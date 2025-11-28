#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{delete, get, post, put, web, Responder};
use log::error;
use uuid::Uuid;

use crate::admin::workflows::models::{
    CreateWorkflowRequest, CreateWorkflowResponse, UpdateWorkflowRequest, WorkflowDetail,
};
use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::RequiredAuth;
use crate::response::ApiResponse;
use crate::response::ValidationViolation;
use r_data_core_core::utils;

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
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    _: RequiredAuth,
) -> impl Responder {
    let uuid = path.into_inner();
    match state.workflow_service().get(uuid).await {
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
    state: web::Data<ApiStateWrapper>,
    body: web::Json<CreateWorkflowRequest>,
    auth: RequiredAuth,
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

    let created = state.workflow_service().create(&body.0, created_by).await;

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
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateWorkflowRequest>,
    auth: RequiredAuth,
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
        .workflow_service()
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
    state: web::Data<ApiStateWrapper>,
    path: web::Path<Uuid>,
    _: RequiredAuth,
) -> impl Responder {
    let uuid = path.into_inner();
    let res = state.workflow_service().delete(uuid).await;
    match res {
        Ok(_) => ApiResponse::<()>::message("Deleted"),
        Err(e) => ApiResponse::<()>::internal_error(&format!("Failed to delete: {}", e)),
    }
}
