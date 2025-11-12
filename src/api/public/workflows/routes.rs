use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use std::result::Result;
use uuid::Uuid;

use crate::api::auth::auth_enum::CombinedRequiredAuth;
use crate::api::ApiState;
use crate::workflow::data::adapters::auth::{AuthConfig, KeyLocation};
use crate::workflow::data::adapters::format::FormatHandler;
use crate::workflow::dsl::DslProgram;

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
#[get("/{uuid}")]
pub async fn get_workflow_data(
    path: web::Path<Uuid>,
    req: HttpRequest,
    state: web::Data<ApiState>,
) -> impl Responder {
    let uuid = path.into_inner();

    // Get workflow config
    let workflow = match state.workflow_service.get(uuid).await {
        Ok(Some(wf)) => wf,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Workflow not found"})),
        Err(e) => {
            log::error!("Failed to get workflow: {}", e);
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Internal server error"}));
        }
    };

    // Only provider workflows can be accessed via GET
    if workflow.kind != crate::workflow::data::WorkflowKind::Provider {
        return HttpResponse::NotFound().json(json!({"error": "Workflow not found"}));
    }

    // Validate pre-shared key if configured (sets extension for CombinedRequiredAuth)
    if let Err(e) = validate_provider_auth(&req, &workflow.config, &state).await {
        log::debug!("Provider pre-shared key auth failed: {}", e);
        return HttpResponse::Unauthorized().json(json!({"error": "Authentication required"}));
    }

    // Use CombinedRequiredAuth to validate JWT/API key (or check pre-shared key extension)
    // Note: We can't use the extractor directly here since we need workflow config first
    // So we manually check the extension set by validate_provider_auth
    let has_pre_shared_key = req.extensions().get::<bool>().copied().unwrap_or(false);
    if !has_pre_shared_key {
        // Try to validate via CombinedRequiredAuth (JWT/API key)
        // We'll create a temporary request to use the extractor
        use crate::api::auth::auth_enum::CombinedRequiredAuth;
        use actix_web::FromRequest;
        let mut payload = actix_web::dev::Payload::None;
        match CombinedRequiredAuth::from_request(&req, &mut payload).await {
            Ok(_) => {
                // Authentication successful
            }
            Err(_) => {
                // Check if pre-shared key was required
                if extract_provider_auth_config(&workflow.config).is_some() {
                    return HttpResponse::Unauthorized()
                        .json(json!({"error": "Authentication required"}));
                }
                // If no pre-shared key required, still need JWT/API key
                return HttpResponse::Unauthorized()
                    .json(json!({"error": "Authentication required"}));
            }
        }
    }

    // Parse DSL program
    let program = match DslProgram::from_config(&workflow.config) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to parse DSL for workflow {}: {}", uuid, e);
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid workflow configuration",
                "details": format!("{}", e),
                "message": "The workflow DSL configuration is invalid or uses an outdated format. Please update the workflow configuration."
            }));
        }
    };

    // Collect input data from entity sources
    let mut input_data = Vec::new();

    // Check if workflow has from.entity sources
    for step in &program.steps {
        if let crate::workflow::dsl::FromDef::Entity {
            entity_definition,
            filter,
            ..
        } = &step.from
        {
            // Query entities based on filter
            if let Some(entity_service) = &state.dynamic_entity_service {
                use serde_json::Value as JsonValue;
                use std::collections::HashMap;

                // Build filter map from EntityFilter
                let mut filter_map = HashMap::new();
                filter_map.insert(
                    filter.field.clone(),
                    JsonValue::String(filter.value.clone()),
                );

                match entity_service
                    .filter_entities(
                        entity_definition,
                        1000, // Limit to 1000 entities
                        0,    // No offset
                        Some(filter_map),
                        None, // No search
                        None, // No custom sort
                        None, // All fields
                    )
                    .await
                {
                    Ok(entities) => {
                        // Convert entities to JSON for workflow execution
                        // DynamicEntity stores all fields in field_data HashMap
                        for entity in entities {
                            // Convert field_data HashMap directly to JSON object
                            let entity_json: serde_json::Value =
                                serde_json::to_value(&entity.field_data)
                                    .unwrap_or_else(|_| json!({}));
                            input_data.push(entity_json);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to query entities: {}", e);
                        return HttpResponse::InternalServerError()
                            .json(json!({"error": "Failed to query source entities"}));
                    }
                }
            } else {
                return HttpResponse::InternalServerError()
                    .json(json!({"error": "Entity service not available"}));
            }
        }
    }

    // If no entity sources, use empty input (for workflows that don't need entity input)
    if input_data.is_empty() {
        input_data.push(json!({}));
    }

    // Execute workflow for each input and collect outputs
    let mut all_outputs = Vec::new();
    for input in input_data {
        match program.execute(&input) {
            Ok(outputs) => {
                all_outputs.extend(outputs);
            }
            Err(e) => {
                log::error!("Failed to execute workflow: {}", e);
                return HttpResponse::InternalServerError()
                    .json(json!({"error": "Failed to execute workflow"}));
            }
        }
    }

    // Find format output (CSV or JSON) and collect all data
    let mut format_outputs = Vec::new();
    let mut format_config = None;
    for (to_def, data) in all_outputs {
        if let crate::workflow::dsl::ToDef::Format { format, output, .. } = to_def {
            if matches!(output, crate::workflow::dsl::OutputMode::Api) {
                if format_config.is_none() {
                    format_config = Some(format.clone());
                }
                format_outputs.push(data);
            }
        }
    }

    if format_outputs.is_empty() || format_config.is_none() {
        return HttpResponse::InternalServerError()
            .json(json!({"error": "No API output format found"}));
    }

    let format = format_config.unwrap();
    let all_data = format_outputs;

    // Serialize based on format
    match format.format_type.as_str() {
        "csv" => {
            let handler = crate::workflow::data::adapters::format::csv::CsvFormatHandler::new();
            match handler.serialize(&all_data, &format.options) {
                Ok(bytes) => {
                    return HttpResponse::Ok().content_type("text/csv").body(bytes);
                }
                Err(e) => {
                    log::error!("Failed to serialize CSV: {}", e);
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to serialize data"}));
                }
            }
        }
        "json" => {
            let handler = crate::workflow::data::adapters::format::json::JsonFormatHandler::new();
            match handler.serialize(&all_data, &format.options) {
                Ok(bytes) => {
                    return HttpResponse::Ok()
                        .content_type("application/json")
                        .body(bytes);
                }
                Err(e) => {
                    log::error!("Failed to serialize JSON: {}", e);
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to serialize data"}));
                }
            }
        }
        _ => {
            return HttpResponse::NotImplemented().json(json!({"error": "Format not supported"}));
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
    state: web::Data<ApiState>,
    _: CombinedRequiredAuth,
) -> impl Responder {
    let uuid = path.into_inner();

    let workflow = match state.workflow_service.get(uuid).await {
        Ok(Some(wf)) => wf,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Workflow not found"})),
        Err(e) => {
            log::error!("Failed to get workflow: {}", e);
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Internal server error"}));
        }
    };

    // Parse DSL to extract metadata
    let program = match DslProgram::from_config(&workflow.config) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to parse DSL for workflow {}: {}", uuid, e);
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid workflow configuration",
                "details": format!("{}", e),
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
        if let crate::workflow::dsl::FromDef::Format { format, source, .. } = &step.from {
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
        if let crate::workflow::dsl::ToDef::Format { format, output, .. } = &step.to {
            formats.push(format.format_type.clone());
            if matches!(output, crate::workflow::dsl::OutputMode::Api) {
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
    state: web::Data<ApiState>,
) -> impl Responder {
    let uuid = path.into_inner();

    // Get workflow
    let workflow = match state.workflow_service.get(uuid).await {
        Ok(Some(wf)) => wf,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Workflow not found"})),
        Err(e) => {
            log::error!("Failed to get workflow: {}", e);
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Internal server error"}));
        }
    };

    // Only consumer workflows can accept POST
    if workflow.kind != crate::workflow::data::WorkflowKind::Consumer {
        return HttpResponse::MethodNotAllowed()
            .json(json!({"error": "This endpoint only accepts POST for consumer workflows"}));
    }

    // Check if workflow has from.api source
    let program = match DslProgram::from_config(&workflow.config) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to parse DSL for workflow {}: {}", uuid, e);
            return HttpResponse::BadRequest().json(json!({
                "error": "Invalid workflow configuration",
                "details": format!("{}", e),
                "message": "The workflow DSL configuration is invalid or uses an outdated format. Please update the workflow configuration."
            }));
        }
    };

    let has_api_source = program.steps.iter().any(|step| {
        if let crate::workflow::dsl::FromDef::Format { source, .. } = &step.from {
            source.source_type == "api"
        } else {
            false
        }
    });

    if !has_api_source {
        return HttpResponse::BadRequest()
            .json(json!({"error": "Workflow does not support API ingestion"}));
    }

    // Create a run and process synchronously
    match state
        .workflow_service
        .run_now_upload_bytes(uuid, &body)
        .await
    {
        Ok((run_uuid, staged_count)) => HttpResponse::Accepted().json(json!({
            "run_uuid": run_uuid,
            "staged_items": staged_count,
            "status": "processing"
        })),
        Err(e) => {
            log::error!("Failed to process workflow: {}", e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to process workflow"}))
        }
    }
}

/// Validate provider authentication (JWT, API key, or pre-shared key)
/// Sets request extension for pre-shared keys so CombinedRequiredAuth can pick it up
async fn validate_provider_auth(
    req: &HttpRequest,
    config: &serde_json::Value,
    _state: &ApiState,
) -> Result<(), String> {
    // Check for pre-shared key in config first
    if let Some(auth_config) = extract_provider_auth_config(config) {
        if let AuthConfig::PreSharedKey {
            key,
            location,
            field_name,
        } = auth_config
        {
            let provided_key = match location {
                KeyLocation::Header => req
                    .headers()
                    .get(&field_name)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string()),
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
