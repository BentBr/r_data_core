#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use std::result::Result;
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::CombinedRequiredAuth;
use r_data_core_workflow::data::adapters::auth::{AuthConfig, KeyLocation};
use r_data_core_workflow::data::adapters::format::FormatHandler;
use r_data_core_workflow::dsl::{DslProgram, FromDef, OutputMode, ToDef};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

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
                    if filter_map.is_empty() { None } else { Some(filter_map) },
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
            .service(get_workflow_data)
            .service(get_workflow_stats)
            .service(post_workflow_ingest),
    );
}

/// Get workflow data (provider workflow)
/// Returns data in the format specified by the workflow config (CSV or JSON)
#[utoipa::path(
    get,
    path = "/api/v1/workflows/{uuid}",
    tag = "workflows",
    params(
        ("uuid" = Uuid, Path, description = "Workflow UUID")
    ),
    responses(
        (status = 200, description = "Workflow data in configured format (CSV or JSON)", content_type = "text/csv,application/json"),
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
) -> impl Responder {
    let uuid = path.into_inner();

    // Get workflow config
    let workflow = match state.workflow_service().get(uuid).await {
        Ok(Some(wf)) => wf,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Workflow not found"})),
        Err(e) => {
            log::error!("Failed to get workflow: {e}");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Internal server error"}));
        }
    };

    // Only provider workflows can be accessed via GET
    if workflow.kind != r_data_core_workflow::data::WorkflowKind::Provider {
        return HttpResponse::NotFound().json(json!({"error": "Workflow not found"}));
    }

    // Validate pre-shared key if configured (sets extension for CombinedRequiredAuth)
    if let Err(e) = validate_provider_auth(&req, &workflow.config, &**state) {
        log::debug!("Provider pre-shared key auth failed: {e}");
        return HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}));
    }

    // Extract pre-shared key status and clone request before any await points
    let has_pre_shared_key = req.extensions().get::<bool>().copied().unwrap_or(false);
    let req_clone = req.clone(); // Clone request for use in async block

    // Use CombinedRequiredAuth to validate JWT/API key (or check pre-shared key extension)
    // Note: We can't use the extractor directly here since we need workflow config first
    // So we manually check the extension set by validate_provider_auth
    if !has_pre_shared_key {
        // Try to validate via CombinedRequiredAuth (JWT/API key)
        // Use cloned request to avoid Send issues
        use crate::auth::auth_enum::CombinedRequiredAuth;
        use actix_web::FromRequest;
        let mut payload = actix_web::dev::Payload::None;
        if CombinedRequiredAuth::from_request(&req_clone, &mut payload)
            .await
            .is_err()
        {
            // Check if pre-shared key was required
            if extract_provider_auth_config(&workflow.config).is_some() {
                return HttpResponse::Unauthorized()
                    .json(json!({"error": "Authentication required"}));
            }
            // If no pre-shared key required, still need JWT/API key
            return HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}));
        }
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

    // Collect input data from entity sources
    let input_data = match collect_entity_input_data(&program, &state).await {
        Ok(data) => data,
        Err(resp) => return resp,
    };

    // Execute workflow and collect format outputs
    let (format_outputs, format_config) =
        match execute_workflow_and_collect_outputs(&program, input_data) {
            Ok(result) => result,
            Err(resp) => return resp,
        };

    if format_outputs.is_empty() || format_config.is_none() {
        return HttpResponse::InternalServerError()
            .json(json!({"error": "No API output format found"}));
    }

    let format = format_config.unwrap();
    let all_data = format_outputs;

    // Serialize based on format
    match format.format_type.as_str() {
        "csv" => {
            let handler =
                r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler::new();
            match handler.serialize(&all_data, &format.options) {
                Ok(bytes) => HttpResponse::Ok().content_type("text/csv").body(bytes),
                Err(e) => {
                    log::error!("Failed to serialize CSV: {e}");
                    HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to serialize data"}))
                }
            }
        }
        "json" => {
            let handler =
                r_data_core_workflow::data::adapters::format::json::JsonFormatHandler::new();
            match handler.serialize(&all_data, &format.options) {
                Ok(bytes) => HttpResponse::Ok()
                    .content_type("application/json")
                    .body(bytes),
                Err(e) => {
                    log::error!("Failed to serialize JSON: {e}");
                    HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to serialize data"}))
                }
            }
        }
        _ => HttpResponse::NotImplemented().json(json!({"error": "Format not supported"})),
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
#[utoipa::path(
    post,
    path = "/api/v1/workflows/{uuid}",
    tag = "workflows",
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
        (status = 404, description = "Workflow not found"),
        (status = 405, description = "Method not allowed - only consumer workflows accept POST"),
        (status = 500, description = "Internal server error")
    )
)]
#[post("/{uuid}")]
pub async fn post_workflow_ingest(
    path: web::Path<Uuid>,
    body: web::Bytes,
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

    // Create a run and process synchronously
    match state
        .workflow_service()
        .run_now_upload_bytes(uuid, &body)
        .await
    {
        Ok((run_uuid, staged_count)) => HttpResponse::Accepted().json(json!({
            "run_uuid": run_uuid,
            "staged_items": staged_count,
            "status": "processing"
        })),
        Err(e) => {
            log::error!("Failed to process workflow: {e}");
            HttpResponse::InternalServerError().json(json!({"error": "Failed to process workflow"}))
        }
    }
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
