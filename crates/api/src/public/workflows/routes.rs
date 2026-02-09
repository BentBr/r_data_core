#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Actix handlers take HttpRequest which is !Send

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::CombinedRequiredAuth;
use r_data_core_core::error::Error;
use r_data_core_workflow::data::adapters::auth::AuthConfig;
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;
use r_data_core_workflow::data::WorkflowKind;
use r_data_core_workflow::dsl::{DslProgram, FromDef, OutputMode, ToDef};
use serde::Deserialize;

use super::helpers::{extract_provider_auth_config, validate_and_authenticate_workflow};
use super::orchestration::{handle_provider_workflow, handle_trigger_consumer_workflow};

#[derive(Deserialize)]
pub(super) struct WorkflowQuery {
    #[serde(default)]
    pub r#async: Option<bool>,
    #[serde(default)]
    pub run_uuid: Option<Uuid>,
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/workflows")
            // Register more specific routes FIRST to avoid route conflicts
            // Routes with additional path segments must be registered before
            // the catch-all /{uuid} route
            .service(trigger_workflow) // /{uuid}/trigger
            .service(get_workflow_stats) // /{uuid}/stats
            .service(get_workflow_data) // /{uuid} (GET)
            .service(post_workflow_ingest), // /{uuid} (POST)
    );
}

/// Get workflow data (Provider workflows only)
/// Returns data in the format specified by the workflow config (CSV or JSON)
/// Supports sync and async modes via ?async=true query parameter
/// Authentication is required (JWT, API key, or pre-shared key)
#[utoipa::path(
    get,
    path = "/api/v1/workflows/{uuid}",
    tag = "workflows",
    summary = "Get workflow data",
    description = "Get data from a Provider workflow. Returns data in the configured format (CSV or JSON). Supports sync and async execution modes.",
    params(
        ("uuid" = Uuid, Path, description = "Workflow UUID"),
        ("async" = Option<bool>, Query, description = "Execute async (202) or sync (200)"),
        ("run_uuid" = Option<Uuid>, Query, description = "Run UUID to poll when async=true")
    ),
    responses(
        (status = 200, description = "Workflow data in configured format (CSV or JSON)", content_type = "text/csv,application/json"),
        (status = 202, description = "Workflow execution queued (use /workflows/{uuid} again to check status)"),
        (status = 401, description = "Unauthorized - authentication required"),
        (status = 404, description = "Workflow not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = []),
        ("preSharedKey" = [])
    )
)]
#[get("/{uuid}")]
pub async fn get_workflow_data(
    path: web::Path<Uuid>,
    req: HttpRequest,
    state: web::Data<ApiStateWrapper>,
    query: web::Query<WorkflowQuery>,
) -> impl Responder {
    let uuid = path.into_inner();

    // Get workflow config and validate auth
    let workflow = match state.workflow_service().get(uuid).await {
        Ok(Some(wf)) => wf,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Workflow not found"})),
        Err(Error::NotFound(_)) => {
            return HttpResponse::NotFound().json(json!({"error": "Workflow not found"}))
        }
        Err(e) => {
            log::error!("Failed to get workflow: {e}");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Internal server error"}));
        }
    };

    // Only Provider workflows can be accessed via GET /api/v1/workflows/{uuid}
    if workflow.kind != WorkflowKind::Provider {
        return HttpResponse::NotFound().json(json!({"error": "Workflow not found"}));
    }

    // Parse DSL program
    let program = match DslProgram::from_config(&workflow.config) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to parse DSL for workflow {uuid}: {e}");
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid workflow configuration",
                "details": format!("{e}"),
                "message": "The workflow DSL configuration is invalid or uses an outdated format. Please update the workflow configuration."
            }));
        }
    };

    handle_provider_workflow(uuid, &req, &workflow, &program, &state, &query).await
}

/// Trigger workflow execution (Consumer workflows with trigger type only)
/// Accepts GET requests to trigger workflow execution at /api/v1/workflows/{uuid}/trigger
/// No data payload - just triggers the workflow to run
/// Supports sync and async modes via ?async=true query parameter
/// Authentication is required (JWT, API key, or pre-shared key)
#[utoipa::path(
    get,
    path = "/api/v1/workflows/{uuid}/trigger",
    tag = "workflows",
    summary = "Trigger workflow execution",
    description = "Trigger a Consumer workflow with trigger type. Accepts GET requests with no data payload. Supports sync and async execution modes.",
    params(
        ("uuid" = Uuid, Path, description = "Workflow UUID"),
        ("async" = Option<bool>, Query, description = "Execute async (202) or sync (200)"),
        ("run_uuid" = Option<Uuid>, Query, description = "Run UUID to poll when async=true")
    ),
    responses(
        (status = 200, description = "Workflow execution completed", body = serde_json::Value),
        (status = 202, description = "Workflow execution queued (use /workflows/{uuid}/trigger again to check status)"),
        (status = 401, description = "Unauthorized - authentication required"),
        (status = 404, description = "Workflow not found or not a Consumer workflow with trigger type"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = []),
        ("preSharedKey" = [])
    )
)]
#[get("/{uuid}/trigger")]
pub async fn trigger_workflow(
    path: web::Path<Uuid>,
    req: HttpRequest,
    state: web::Data<ApiStateWrapper>,
    query: web::Query<WorkflowQuery>,
) -> impl Responder {
    let uuid = path.into_inner();

    // Get workflow config and validate auth
    let workflow = match state.workflow_service().get(uuid).await {
        Ok(Some(wf)) => wf,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Workflow not found"})),
        Err(Error::NotFound(_)) => {
            return HttpResponse::NotFound().json(json!({"error": "Workflow not found"}))
        }
        Err(e) => {
            log::error!("Failed to get workflow: {e}");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Internal server error"}));
        }
    };

    // Only Consumer workflows can be triggered
    if workflow.kind != WorkflowKind::Consumer {
        return HttpResponse::NotFound().json(json!({
            "error": "Workflow not found",
            "message": "Only Consumer workflows can be triggered via this endpoint"
        }));
    }

    // Parse DSL program to check for trigger type
    let program = match DslProgram::from_config(&workflow.config) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to parse DSL for workflow {uuid}: {e}");
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid workflow configuration",
                "details": format!("{e}"),
                "message": "The workflow DSL configuration is invalid or uses an outdated format. Please update the workflow configuration."
            }));
        }
    };

    // Check for trigger type in first step
    let has_trigger = program
        .steps
        .first()
        .is_some_and(|step| matches!(step.from, FromDef::Trigger { .. }));

    if !has_trigger {
        return HttpResponse::NotFound().json(json!({
            "error": "Workflow not found",
            "message": "This workflow does not have a trigger type in the first step"
        }));
    }

    handle_trigger_consumer_workflow(uuid, &req, &workflow, &state, &query).await
}

/// Get workflow stats/metadata
/// Returns metadata about the workflow including format, auth requirements, and field mappings
#[utoipa::path(
    get,
    path = "/api/v1/workflows/{uuid}/stats",
    tag = "workflows",
    params(
        ("uuid" = Uuid, Path, description = "Workflow UUID")
    ),
    responses(
        (status = 200, description = "Workflow metadata", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Workflow not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = [])
    )
)]
#[get("/{uuid}/stats")]
pub async fn get_workflow_stats(
    path: web::Path<Uuid>,
    state: web::Data<ApiStateWrapper>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let uuid = path.into_inner();

    let workflow = match state.workflow_service().get(uuid).await {
        Ok(Some(wf)) => wf,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Workflow not found"})),
        Err(e) => {
            log::error!("Failed to get workflow: {e}");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Internal server error"}));
        }
    };

    // Parse DSL to extract metadata
    let program = match DslProgram::from_config(&workflow.config) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to parse DSL for workflow {uuid}: {e}");
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid workflow configuration",
                "details": format!("{e}"),
                "message": "The workflow DSL configuration is invalid or uses an outdated format. Please update the workflow configuration."
            }));
        }
    };

    // Extract metadata from steps
    let mut formats = Vec::new();
    let mut auth_required = false;
    let mut auth_types = Vec::new();

    for step in &program.steps {
        // Check from format
        if let FromDef::Format { format, source, .. } = &step.from {
            formats.push(format.format_type.clone());
            if let Some(auth_config) = &source.auth {
                auth_required = true;
                auth_types.push(match auth_config {
                    AuthConfig::None => "none".to_string(),
                    AuthConfig::ApiKey { .. } => "api_key".to_string(),
                    AuthConfig::BasicAuth { .. } => "basic_auth".to_string(),
                    AuthConfig::PreSharedKey { .. } => "pre_shared_key".to_string(),
                });
            }
        }
        // Check to format
        if let ToDef::Format { format, output, .. } = &step.to {
            formats.push(format.format_type.clone());
            if matches!(output, OutputMode::Api) {
                // Provider workflow endpoint - check for pre-shared key requirement
                if extract_provider_auth_config(&workflow.config).is_some() {
                    auth_required = true;
                    auth_types.push("pre_shared_key".to_string());
                }
            }
        }
    }

    HttpResponse::Ok().json(json!({
        "uuid": uuid,
        "name": workflow.name,
        "formats": formats,
        "auth_required": auth_required,
        "auth_types": auth_types,
    }))
}

/// POST endpoint for ingesting data (consumer workflow)
/// Synchronously triggers workflow processing for consumer workflows with from.api source
/// Authentication is required (JWT, API key, or pre-shared key)
#[utoipa::path(
    post,
    path = "/api/v1/workflows/{uuid}",
    tag = "workflows",
    summary = "Ingest data into workflow",
    description = "POST endpoint for ingesting data into a Consumer workflow with from.api source. Accepts CSV or JSON data payload.",
    params(
        ("uuid" = Uuid, Path, description = "Workflow UUID")
    ),
    request_body(
        content = String,
        description = "CSV or JSON data to ingest",
        content_type = "text/csv,application/json"
    ),
    responses(
        (status = 202, description = "Data accepted and processing started", body = serde_json::Value),
        (status = 400, description = "Bad request - workflow does not support API ingestion"),
        (status = 401, description = "Unauthorized - authentication required"),
        (status = 404, description = "Workflow not found"),
        (status = 405, description = "Method not allowed - only consumer workflows accept POST"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = []),
        ("apiKey" = []),
        ("preSharedKey" = [])
    )
)]
#[post("/{uuid}")]
pub async fn post_workflow_ingest(
    path: web::Path<Uuid>,
    body: web::Bytes,
    req: HttpRequest,
    state: web::Data<ApiStateWrapper>,
) -> impl Responder {
    let uuid = path.into_inner();

    // Get workflow
    let workflow = match state.workflow_service().get(uuid).await {
        Ok(Some(wf)) => wf,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Workflow not found"})),
        Err(e) => {
            log::error!("Failed to get workflow: {e}");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Internal server error"}));
        }
    };

    // Validate authentication (required for all workflows)
    if let Err(resp) = validate_and_authenticate_workflow(&req, &workflow, &state).await {
        return resp;
    }

    // Only consumer workflows can accept POST
    if workflow.kind != WorkflowKind::Consumer {
        return HttpResponse::MethodNotAllowed()
            .json(json!({"error": "This endpoint only accepts POST for consumer workflows"}));
    }

    // Check if workflow is enabled
    if !workflow.enabled {
        return HttpResponse::ServiceUnavailable().json(json!({
            "error": "Workflow is not enabled",
            "message": "This workflow is currently disabled and cannot accept data"
        }));
    }

    // Check if workflow has from.api source (without endpoint field - meaning it accepts POST)
    let program = match DslProgram::from_config(&workflow.config) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to parse DSL for workflow {uuid}: {e}");
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid workflow configuration",
                "details": format!("{e}"),
                "message": "The workflow DSL configuration is invalid or uses an outdated format. Please update the workflow configuration."
            }));
        }
    };

    // Check for from.api source WITHOUT endpoint field (accepts POST)
    let has_api_source_accepting_post = program.steps.iter().any(|step| {
        if let FromDef::Format { source, .. } = &step.from {
            source.source_type == "api" && source.config.get("endpoint").is_none()
        } else {
            false
        }
    });

    if !has_api_source_accepting_post {
        return HttpResponse::BadRequest()
            .json(json!({"error": "Workflow does not support API ingestion", "message": "This workflow must have a 'from.api' source type (without endpoint field) to accept POST data"}));
    }

    // Create a run and stage items
    let (run_uuid, staged_count) = match state
        .workflow_service()
        .run_now_upload_bytes(uuid, &body)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            log::error!("Failed to stage workflow data: {e}");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to process workflow"}));
        }
    };

    // Enqueue the job to Redis for worker processing
    if let Err(e) = state
        .queue()
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: uuid,
            trigger_id: Some(run_uuid),
        })
        .await
    {
        log::error!("Failed to enqueue fetch job for workflow {uuid} (run: {run_uuid}): {e}");
        // Return warning but don't fail - items are staged
        return HttpResponse::Accepted().json(json!({
            "run_uuid": run_uuid,
            "staged_items": staged_count,
            "status": "queued",
            "warning": "Items staged but job enqueue failed - processing may be delayed"
        }));
    }

    log::info!("Successfully enqueued workflow {uuid} (run: {run_uuid}, staged: {staged_count})");
    HttpResponse::Accepted().json(json!({
        "run_uuid": run_uuid,
        "staged_items": staged_count,
        "status": "queued"
    }))
}
