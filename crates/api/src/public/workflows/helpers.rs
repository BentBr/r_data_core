#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Async functions called from Actix handlers which are !Send

use actix_web::{dev::Payload, web, FromRequest, HttpMessage, HttpRequest, HttpResponse};
use serde_json::json;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::result::Result;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::auth_enum::CombinedRequiredAuth;
use r_data_core_core::entity_jwt;
use r_data_core_workflow::data::adapters::auth::{AuthConfig, KeyLocation};
use r_data_core_workflow::dsl::{DslProgram, FormatConfig, FromDef, OutputMode, ToDef};

/// Collect input data from entity sources in workflow steps
pub(super) async fn collect_entity_input_data(
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
pub(super) fn execute_workflow_and_collect_outputs(
    program: &DslProgram,
    input_data: Vec<JsonValue>,
) -> Result<(Vec<JsonValue>, Option<FormatConfig>), HttpResponse> {
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

pub(super) async fn validate_and_authenticate_workflow(
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
        let mut payload = Payload::None;
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

/// Validate provider authentication (JWT, API key, or pre-shared key)
/// Sets request extension for pre-shared keys so `CombinedRequiredAuth` can pick it up
fn validate_provider_auth(
    req: &HttpRequest,
    config: &serde_json::Value,
    state: &dyn ApiStateTrait,
) -> Result<(), String> {
    match extract_provider_auth_config(config) {
        Some(AuthConfig::PreSharedKey {
            key,
            location,
            field_name,
        }) => {
            let provided_key = match location {
                KeyLocation::Header => req
                    .headers()
                    .get(&field_name)
                    .and_then(|v| v.to_str().ok())
                    .map(std::string::ToString::to_string),
                KeyLocation::Body => None,
            };

            if let Some(provided) = provided_key {
                if provided == key {
                    req.extensions_mut().insert(true);
                    return Ok(());
                }
            }
            Err("Invalid pre-shared key".to_string())
        }
        Some(AuthConfig::EntityJwt { required_claims }) => {
            // Extract Bearer token
            let token = req
                .headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .ok_or_else(|| "Missing entity JWT Bearer token".to_string())?;

            // Verify entity JWT
            let claims = entity_jwt::verify_entity_jwt(token, state.jwt_secret()).map_err(|e| {
                log::debug!("Entity JWT verification failed: {e}");
                "Invalid entity JWT".to_string()
            })?;

            // Check required claims if configured
            if let Some(ref required) = required_claims {
                for (claim_path, expected_value) in required {
                    let actual = resolve_claim_path(&claims, claim_path);
                    if actual.as_ref() != Some(expected_value) {
                        return Err(format!("Required claim '{claim_path}' mismatch"));
                    }
                }
            }

            // Store claims in request extensions and mark as authenticated
            req.extensions_mut().insert(claims);
            req.extensions_mut().insert(true);
            Ok(())
        }
        _ => {
            // No provider-specific auth, fall through to JWT/API key via CombinedRequiredAuth
            Ok(())
        }
    }
}

/// Resolve a dotted claim path against `EntityAuthClaims`.
/// Supports `entity_type`, `sub`, and `extra.{key}` paths.
fn resolve_claim_path(
    claims: &entity_jwt::EntityAuthClaims,
    path: &str,
) -> Option<serde_json::Value> {
    match path {
        "sub" => Some(serde_json::Value::String(claims.sub.clone())),
        "entity_type" => Some(serde_json::Value::String(claims.entity_type.clone())),
        "iss" => Some(serde_json::Value::String(claims.iss.clone())),
        _ if path.starts_with("extra.") => {
            let key = &path["extra.".len()..];
            claims.extra.get(key).cloned()
        }
        _ => None,
    }
}

/// Extract provider auth config from workflow config
pub(super) fn extract_provider_auth_config(config: &serde_json::Value) -> Option<AuthConfig> {
    // Look for auth config in the provider-specific section
    config
        .get("provider_auth")
        .and_then(|v| serde_json::from_value::<AuthConfig>(v.clone()).ok())
}
