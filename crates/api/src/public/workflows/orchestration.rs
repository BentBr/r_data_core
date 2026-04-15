#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::future_not_send)] // Async functions called from Actix handlers which are !Send

use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use r_data_core_core::error::Error;
use r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler;
use r_data_core_workflow::data::adapters::format::json::JsonFormatHandler;
use r_data_core_workflow::data::adapters::format::FormatHandler;
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;
use r_data_core_workflow::dsl::{DslProgram, FormatConfig};

use super::helpers::{
    collect_entity_input_data, execute_workflow_and_collect_outputs,
    validate_and_authenticate_workflow,
};
use super::routes::WorkflowQuery;

pub(super) async fn handle_provider_workflow(
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
        let format = format_config.unwrap_or_else(|| FormatConfig {
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

pub(super) async fn handle_trigger_consumer_workflow(
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

/// Handle an inline auth workflow: process the payload synchronously and return the result.
///
/// Maps domain errors to HTTP status codes:
/// - `Error::Auth` → 401 Unauthorized
/// - `Error::Validation` → 400 Bad Request
/// - other → 500 Internal Server Error
pub(super) async fn handle_inline_auth_workflow(
    uuid: Uuid,
    payload: &JsonValue,
    state: &web::Data<ApiStateWrapper>,
) -> HttpResponse {
    match state
        .workflow_service()
        .process_payload_inline(uuid, payload)
        .await
    {
        Ok(output) => HttpResponse::Ok().json(output),
        Err(Error::Auth(msg)) => HttpResponse::Unauthorized().json(json!({"error": msg})),
        Err(Error::Validation(msg)) => HttpResponse::BadRequest().json(json!({"error": msg})),
        Err(e) => {
            log::error!("Inline auth workflow {uuid} failed: {e}");
            HttpResponse::InternalServerError().json(json!({"error": "Internal server error"}))
        }
    }
}

async fn serialize_api_output(
    format: FormatConfig,
    all_data: &[JsonValue],
    run_uuid: Uuid,
    state: &web::Data<ApiStateWrapper>,
) -> HttpResponse {
    match format.format_type.as_str() {
        "csv" => {
            let handler = CsvFormatHandler::new();
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
            let handler = JsonFormatHandler::new();
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
