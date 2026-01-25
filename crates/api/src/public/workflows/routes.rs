#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use std::collections::HashMap;
use std::result::Result;
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::CombinedRequiredAuth;
use r_data_core_core::error::Error;
use r_data_core_workflow::data::adapters::auth::{AuthConfig, KeyLocation};
use r_data_core_workflow::data::adapters::format::FormatHandler;
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;
use r_data_core_workflow::dsl::{DslProgram, FromDef, OutputMode, ToDef};
use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Deserialize)]
struct WorkflowQuery {
    #[serde(default)]
    r#async: Option<bool>,
    #[serde(default)]
    run_uuid: Option<Uuid>,
}

/// Collect input data from entity sources in workflow steps
async fn collect_entity_input_data(
    program: &DslProgram,
    state: &ApiStateWrapper,
) -> Result<Vec<JsonValue>, HttpResponse> {
    let mut input_data = Vec::new();

    for step in &program.steps {
        if let FromDef::Entity {
            entity_definition,
            filter,
            ..
        } = &step.from
        {
            let Some(entity_service) = state.dynamic_entity_service() else {
                return Err(HttpResponse::InternalServerError()
                    .json(json!({"error": "Entity service not available"})));
            };

            let mut filter_map = HashMap::new();
            let mut operators_map = HashMap::new();

            if let Some(filter) = filter {
                // Handle IN/NOT IN operators - value should be an array
                let filter_value = if filter.operator == "IN" || filter.operator == "NOT IN" {
                    // Try to parse value as JSON array, otherwise wrap in array
                    match serde_json::from_str::<JsonValue>(&filter.value) {
                        Ok(JsonValue::Array(_)) => serde_json::from_str(&filter.value)
                            .unwrap_or_else(|_| json!([filter.value])),
                        _ => json!([filter.value]),
                    }
                } else {
                    // Try to parse as a number for numeric comparisons, otherwise use as string
                    // This allows numeric string values like "15" to be compared with integer fields
                    filter.value.parse::<i64>().map_or_else(
                        |_| {
                            filter.value.parse::<f64>().map_or_else(
                                |_| JsonValue::String(filter.value.clone()),
                                |num| json!(num),
                            )
                        },
                        |num| json!(num),
                    )
                };
                filter_map.insert(filter.field.clone(), filter_value);
                operators_map.insert(filter.field.clone(), filter.operator.clone());
            }

            let entities = entity_service
                .filter_entities_with_operators(
                    entity_definition,
                    1000,
                    0,
                    if filter_map.is_empty() {
                        None
                    } else {
                        Some(filter_map)
                    },
                    if operators_map.is_empty() {
                        None
                    } else {
                        Some(operators_map)
                    },
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to query entities: {e}");
                    HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to query source entities"}))
                })?;

            for entity in entities {
                let entity_json: JsonValue =
                    serde_json::to_value(&entity.field_data).unwrap_or_else(|_| json!({}));
                input_data.push(entity_json);
            }
        }
    }

    if input_data.is_empty() {
        input_data.push(json!({}));
    }

    Ok(input_data)
}

/// Execute workflow and collect format outputs
///
/// # Errors
///
/// Returns `HttpResponse::InternalServerError` if:
/// - Failed to execute workflow
fn execute_workflow_and_collect_outputs(
    program: &DslProgram,
    input_data: Vec<JsonValue>,
) -> Result<
    (
        Vec<JsonValue>,
        Option<r_data_core_workflow::dsl::FormatConfig>,
    ),
    HttpResponse,
> {
    let mut all_outputs = Vec::new();
    for input in input_data {
        let outputs = program.execute(&input).map_err(|e| {
            log::error!("Failed to execute workflow: {e}");
            HttpResponse::InternalServerError().json(json!({"error": "Failed to execute workflow"}))
        })?;
        all_outputs.extend(outputs);
    }

    let mut format_outputs = Vec::new();
    let mut format_config = None;
    for (to_def, data) in all_outputs {
        if let ToDef::Format { format, output, .. } = to_def {
            if matches!(output, OutputMode::Api) {
                if format_config.is_none() {
                    format_config = Some(format.clone());
                }
                format_outputs.push(data);
            }
        }
    }

    Ok((format_outputs, format_config))
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
#[allow(clippy::future_not_send)] // HttpRequest is not Send, but Actix Web handles this internally
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
    if workflow.kind != r_data_core_workflow::data::WorkflowKind::Provider {
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
#[allow(clippy::future_not_send)] // HttpRequest is not Send, but Actix Web handles this internally
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
    if workflow.kind != r_data_core_workflow::data::WorkflowKind::Consumer {
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
    let has_trigger = program.steps.first().is_some_and(|step| {
        matches!(
            step.from,
            r_data_core_workflow::dsl::FromDef::Trigger { .. }
        )
    });

    if !has_trigger {
        return HttpResponse::NotFound().json(json!({
            "error": "Workflow not found",
            "message": "This workflow does not have a trigger type in the first step"
        }));
    }

    handle_trigger_consumer_workflow(uuid, &req, &workflow, &state, &query).await
}

#[allow(clippy::future_not_send)]
async fn handle_trigger_consumer_workflow(
    uuid: Uuid,
    req: &HttpRequest,
    workflow: &r_data_core_workflow::data::Workflow,
    state: &web::Data<ApiStateWrapper>,
    query: &web::Query<WorkflowQuery>,
) -> HttpResponse {
    // Check if workflow is enabled
    if !workflow.enabled {
        return HttpResponse::ServiceUnavailable().json(json!({
            "error": "Workflow is not enabled",
            "message": "This workflow is currently disabled and cannot be triggered"
        }));
    }

    // Validate authentication (required for all workflows)
    if let Err(resp) = validate_and_authenticate_workflow(req, workflow, state).await {
        return resp;
    }

    // Create a run
    let run_uuid = match state.workflow_service().enqueue_run(uuid).await {
        Ok(run_uuid) => run_uuid,
        Err(Error::NotFound(msg)) => {
            log::error!("Workflow not found: {msg}");
            return HttpResponse::NotFound().json(json!({"error": "Workflow not found"}));
        }
        Err(e) => {
            log::error!("Failed to enqueue run: {e}");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to enqueue workflow run"}));
        }
    };
    let _ = state.workflow_service().mark_run_running(run_uuid).await;

    let async_mode = query.r#async.unwrap_or(false);
    let run_uuid_param = query.run_uuid;

    if async_mode {
        if let Some(resp) = match handle_async_get(uuid, run_uuid_param, state).await {
            Ok(resp_opt) => resp_opt,
            Err(resp) => return resp,
        } {
            return resp;
        }
    }

    // For trigger source, fetch and stage from config (will skip trigger source, fetch from uri/entity sources)
    // If no items are staged, stage an empty item to trigger processing
    let fetch_staged = match state
        .workflow_service()
        .fetch_and_stage_from_config(uuid, run_uuid)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            log::error!("Failed to fetch and stage from config: {e}");
            let _ = state
                .workflow_service()
                .mark_run_failure(run_uuid, "Failed to fetch from config")
                .await;
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to fetch workflow data"}));
        }
    };

    // If no items were staged (e.g., only trigger source, no uri/entity sources), stage empty item to trigger processing
    let staged = if fetch_staged == 0 {
        match state
            .workflow_service()
            .stage_raw_items(uuid, run_uuid, vec![serde_json::json!({})])
            .await
        {
            Ok(count) => count,
            Err(e) => {
                log::error!("Failed to stage trigger item: {e}");
                let _ = state
                    .workflow_service()
                    .mark_run_failure(run_uuid, "Failed to stage trigger item")
                    .await;
                return HttpResponse::InternalServerError()
                    .json(json!({"error": "Failed to stage workflow trigger"}));
            }
        }
    } else {
        0
    };

    // Process staged items
    match state
        .workflow_service()
        .process_staged_items(uuid, run_uuid)
        .await
    {
        Ok((processed, failed)) => {
            let _ = state
                .workflow_service()
                .mark_run_success(run_uuid, processed, failed)
                .await;
            HttpResponse::Accepted().json(json!({
                "run_uuid": run_uuid,
                "staged_items": fetch_staged + staged,
                "processed": processed,
                "failed": failed,
                "status": "completed"
            }))
        }
        Err(e) => {
            log::error!("Failed to process workflow: {e}");
            let _ = state
                .workflow_service()
                .mark_run_failure(run_uuid, &format!("Processing failed: {e}"))
                .await;
            HttpResponse::InternalServerError().json(json!({
                "error": "Failed to process workflow",
                "run_uuid": run_uuid
            }))
        }
    }
}

#[allow(clippy::future_not_send)]
async fn handle_provider_workflow(
    uuid: Uuid,
    req: &HttpRequest,
    workflow: &r_data_core_workflow::data::Workflow,
    program: &DslProgram,
    state: &web::Data<ApiStateWrapper>,
    query: &web::Query<WorkflowQuery>,
) -> HttpResponse {
    // Existing Provider workflow behavior
    if let Err(resp) = validate_and_authenticate_workflow(req, workflow, state).await {
        return resp;
    }

    // Create a run and mark running for logging/history
    let run_uuid = match state.workflow_service().enqueue_run(uuid).await {
        Ok(run_uuid) => run_uuid,
        Err(Error::NotFound(msg)) => {
            log::error!("Workflow not found: {msg}");
            return HttpResponse::NotFound().json(json!({"error": "Workflow not found"}));
        }
        Err(e) => {
            log::error!("Failed to enqueue run: {e}");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to enqueue workflow run"}));
        }
    };
    let _ = state.workflow_service().mark_run_running(run_uuid).await;

    let async_mode = query.r#async.unwrap_or(false);
    let run_uuid_param = query.run_uuid;

    if async_mode {
        if let Some(resp) = match handle_async_get(uuid, run_uuid_param, state).await {
            Ok(resp_opt) => resp_opt,
            Err(resp) => return resp,
        } {
            return resp;
        }
    }

    // Collect input data from entity sources (sync execution path)
    let input_data = match collect_entity_input_data(program, state).await {
        Ok(data) => data,
        Err(resp) => {
            let _ = state
                .workflow_service()
                .mark_run_failure(run_uuid, "Failed to fetch entities")
                .await;
            return resp;
        }
    };

    // Log fetch result (including zero entities)
    let entity_count = i64::try_from(input_data.len()).unwrap_or(0);
    let _ = state
        .workflow_service()
        .insert_run_log(
            run_uuid,
            "info",
            &format!("Fetched {entity_count} entities for API export"),
            Some(json!({ "entity_count": entity_count })),
        )
        .await;

    // Execute workflow and collect format outputs
    let (format_outputs, format_config) =
        match execute_workflow_and_collect_outputs(program, input_data) {
            Ok(result) => result,
            Err(resp) => {
                let _ = state
                    .workflow_service()
                    .mark_run_failure(run_uuid, "Workflow execution failed")
                    .await;
                return resp;
            }
        };

    // Handle empty results - return 200 with empty data instead of error
    if format_outputs.is_empty() {
        let format = format_config.unwrap_or_else(|| r_data_core_workflow::dsl::FormatConfig {
            format_type: "json".to_string(),
            options: json!({}),
        });

        // Return empty response based on format
        let response = serialize_api_output(format, &[], run_uuid, state).await;

        // Mark run success with 0 processed
        let _ = state
            .workflow_service()
            .mark_run_success(run_uuid, 0, 0)
            .await;

        return response;
    }

    let Some(format) = format_config else {
        let _ = state
            .workflow_service()
            .mark_run_failure(run_uuid, "No API output format found")
            .await;
        return HttpResponse::InternalServerError()
            .json(json!({"error": "No API output format found"}));
    };
    let all_data = format_outputs;

    // Serialize based on format
    let response = serialize_api_output(format, &all_data, run_uuid, state).await;

    // Mark run success with counts (processed = entity_count)
    let _ = state
        .workflow_service()
        .mark_run_success(run_uuid, entity_count, 0)
        .await;

    response
}

#[allow(clippy::future_not_send)]
async fn validate_and_authenticate_workflow(
    req: &HttpRequest,
    workflow: &r_data_core_workflow::data::Workflow,
    state: &web::Data<ApiStateWrapper>,
) -> Result<(), HttpResponse> {
    // Authentication is required for all workflows (both Provider and Consumer)

    // Validate pre-shared key if configured (sets extension for CombinedRequiredAuth)
    if let Err(e) = validate_provider_auth(req, &workflow.config, &***state) {
        log::debug!("Provider pre-shared key auth failed: {e}");
        return Err(HttpResponse::Unauthorized().json(json!({"error": "Authentication required"})));
    }

    // Extract pre-shared key status and clone request before any await points
    let has_pre_shared_key = req.extensions().get::<bool>().copied().unwrap_or(false);
    let req_clone = req.clone(); // Clone request for use in async block

    // Use CombinedRequiredAuth to validate JWT/API key (or check pre-shared key extension)
    if !has_pre_shared_key {
        use crate::auth::auth_enum::CombinedRequiredAuth;
        use actix_web::FromRequest;
        let mut payload = actix_web::dev::Payload::None;
        if CombinedRequiredAuth::from_request(&req_clone, &mut payload)
            .await
            .is_err()
        {
            // Check if pre-shared key was required
            if extract_provider_auth_config(&workflow.config).is_some() {
                return Err(
                    HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
                );
            }
            // If no pre-shared key required, still need JWT/API key
            return Err(
                HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}))
            );
        }
    }

    Ok(())
}

async fn serialize_api_output(
    format: r_data_core_workflow::dsl::FormatConfig,
    all_data: &[JsonValue],
    run_uuid: Uuid,
    state: &web::Data<ApiStateWrapper>,
) -> HttpResponse {
    match format.format_type.as_str() {
        "csv" => {
            let handler =
                r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler::new();
            match handler.serialize(all_data, &format.options) {
                Ok(bytes) => HttpResponse::Ok().content_type("text/csv").body(bytes),
                Err(e) => {
                    log::error!("Failed to serialize CSV: {e}");
                    let _ = state
                        .workflow_service()
                        .mark_run_failure(run_uuid, "Failed to serialize data (csv)")
                        .await;
                    HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to serialize data"}))
                }
            }
        }
        "json" => {
            let handler =
                r_data_core_workflow::data::adapters::format::json::JsonFormatHandler::new();
            match handler.serialize(all_data, &format.options) {
                Ok(bytes) => HttpResponse::Ok()
                    .content_type("application/json")
                    .body(bytes),
                Err(e) => {
                    log::error!("Failed to serialize JSON: {e}");
                    let _ = state
                        .workflow_service()
                        .mark_run_failure(run_uuid, "Failed to serialize data (json)")
                        .await;
                    HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to serialize data"}))
                }
            }
        }
        other => {
            log::error!("Unsupported format: {other}");
            let _ = state
                .workflow_service()
                .mark_run_failure(run_uuid, "Unsupported format")
                .await;
            HttpResponse::InternalServerError().json(json!({"error": "Unsupported format"}))
        }
    }
}

async fn enqueue_run_for_api(
    workflow_uuid: Uuid,
    state: &web::Data<ApiStateWrapper>,
) -> Result<Uuid, HttpResponse> {
    let run_uuid = match state.workflow_service().enqueue_run(workflow_uuid).await {
        Ok(run_uuid) => run_uuid,
        Err(e) => {
            log::error!("Failed to enqueue run: {e}");
            return Err(HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to enqueue workflow run"})));
        }
    };
    // worker will pick it up via queue
    if let Err(e) = state
        .queue()
        .enqueue_fetch(FetchAndStageJob {
            workflow_id: workflow_uuid,
            trigger_id: Some(run_uuid),
        })
        .await
    {
        log::error!(
            "Failed to enqueue fetch job for workflow {workflow_uuid} (run: {run_uuid}): {e}"
        );
        return Err(HttpResponse::InternalServerError()
            .json(json!({"error": "Failed to enqueue workflow job"})));
    }
    Ok(run_uuid)
}

async fn handle_async_get(
    workflow_uuid: Uuid,
    run_uuid_param: Option<Uuid>,
    state: &web::Data<ApiStateWrapper>,
) -> Result<Option<HttpResponse>, HttpResponse> {
    // If no run_uuid provided, enqueue and return queued
    let Some(run_uuid) = run_uuid_param else {
        let run_uuid = enqueue_run_for_api(workflow_uuid, state).await?;
        return Ok(Some(HttpResponse::Accepted().json(json!({
            "status": "queued",
            "run_uuid": run_uuid,
            "message": "Workflow run enqueued"
        }))));
    };
    match state.workflow_service().get_run_status(run_uuid).await {
        Ok(Some(status)) => {
            if status == "queued" || status == "running" {
                return Ok(Some(HttpResponse::Ok().json(json!({
                    "status": status,
                    "run_uuid": run_uuid
                }))));
            }
            if status == "failed" || status == "cancelled" {
                return Ok(Some(HttpResponse::Ok().json(json!({
                    "status": status,
                    "run_uuid": run_uuid,
                    "error": "Workflow run did not complete successfully"
                }))));
            }
            // status == success: fall through to execute synchronously to return data, without enqueueing
            Ok(None)
        }
        Ok(None) => {
            Err(HttpResponse::NotFound()
                .json(json!({"error": "Run not found", "run_uuid": run_uuid})))
        }
        Err(e) => {
            log::error!("Failed to get run status: {e}");
            Err(HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to get run status"})))
        }
    }
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
        if let r_data_core_workflow::dsl::FromDef::Format { format, source, .. } = &step.from {
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
        if let r_data_core_workflow::dsl::ToDef::Format { format, output, .. } = &step.to {
            formats.push(format.format_type.clone());
            if matches!(output, r_data_core_workflow::dsl::OutputMode::Api) {
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
#[allow(clippy::future_not_send)] // HttpRequest is not Send, but Actix Web handles this internally
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
    if workflow.kind != r_data_core_workflow::data::WorkflowKind::Consumer {
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
        if let r_data_core_workflow::dsl::FromDef::Format { source, .. } = &step.from {
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

/// Validate provider authentication (JWT, API key, or pre-shared key)
/// Sets request extension for pre-shared keys so `CombinedRequiredAuth` can pick it up
#[allow(clippy::unused_async)] // May need async in future
fn validate_provider_auth(
    req: &HttpRequest,
    config: &serde_json::Value,
    _state: &dyn ApiStateTrait,
) -> Result<(), String> {
    // Check for pre-shared key in config first
    if let Some(AuthConfig::PreSharedKey {
        key,
        location,
        field_name,
    }) = extract_provider_auth_config(config)
    {
        let provided_key = match location {
            KeyLocation::Header => req
                .headers()
                .get(&field_name)
                .and_then(|v| v.to_str().ok())
                .map(std::string::ToString::to_string),
            KeyLocation::Body => {
                // Body extraction would need to be done in the route handler
                // For now, we'll check header only
                None
            }
        };

        if let Some(provided) = provided_key {
            if provided == key {
                // Set extension so CombinedRequiredAuth can pick it up
                req.extensions_mut().insert(true);
                return Ok(());
            }
        }
        // Pre-shared key was required but invalid
        return Err("Invalid pre-shared key".to_string());
    }

    // Fall back to JWT/API key via CombinedRequiredAuth
    // We'll let CombinedRequiredAuth handle this, so return Ok here
    // The actual validation happens in the route handler using CombinedRequiredAuth extractor
    Ok(())
}

/// Extract provider auth config from workflow config
fn extract_provider_auth_config(config: &serde_json::Value) -> Option<AuthConfig> {
    // Look for auth config in provider-specific section
    config
        .get("provider_auth")
        .and_then(|v| serde_json::from_value::<AuthConfig>(v.clone()).ok())
}
