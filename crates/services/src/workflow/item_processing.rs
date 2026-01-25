#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::output_handling::{handle_entity_output, handle_format_push_output};
use crate::workflow::transform_execution::execute_async_transform;
use r_data_core_persistence::WorkflowRepositoryTrait;
use r_data_core_workflow::dsl::{DslProgram, ToDef, Transform};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

/// Process a single item through the workflow DSL
///
/// Uses step-by-step execution to properly interleave async transforms (like `ResolveEntityPath`)
/// with sync transforms (like `BuildPath`) that may depend on their results.
///
/// # Errors
/// Returns an error if processing fails
pub async fn process_single_item(
    program: &DslProgram,
    payload: &JsonValue,
    item_uuid: Uuid,
    run_uuid: Uuid,
    versioning_disabled: bool,
    dynamic_entity_service: Option<&DynamicEntityService>,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> r_data_core_core::error::Result<bool> {
    match execute_steps_with_async_transforms(
        program,
        payload,
        item_uuid,
        run_uuid,
        dynamic_entity_service,
        repo,
    )
    .await
    {
        Ok(outputs) => {
            process_outputs(
                outputs,
                payload,
                item_uuid,
                run_uuid,
                versioning_disabled,
                dynamic_entity_service,
                repo,
            )
            .await
        }
        Err(e) => handle_execution_error(e, item_uuid, run_uuid, repo).await,
    }
}

/// Execute workflow steps one at a time, running async transforms between steps.
///
/// This ensures that async transforms (like `ResolveEntityPath`) complete and inject
/// their results into the data context before subsequent steps that depend on them
/// (like `BuildPath`) execute.
async fn execute_steps_with_async_transforms(
    program: &DslProgram,
    payload: &JsonValue,
    item_uuid: Uuid,
    run_uuid: Uuid,
    dynamic_entity_service: Option<&DynamicEntityService>,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> r_data_core_core::error::Result<Vec<(ToDef, JsonValue)>> {
    let mut results: Vec<(ToDef, JsonValue)> = Vec::new();
    let mut previous_step_output: Option<JsonValue> = None;

    for step_idx in 0..program.steps.len() {
        // Step 1: Prepare the step (normalize input, apply sync transforms except BuildPath)
        let (mut normalized, transform) =
            program.prepare_step(step_idx, payload, previous_step_output.as_ref())?;

        // Step 2: Execute async transforms if needed (ResolveEntityPath, GetOrCreateEntity)
        if matches!(
            transform,
            Transform::ResolveEntityPath(_) | Transform::GetOrCreateEntity(_)
        ) {
            if let Some(de_service) = dynamic_entity_service {
                if let Err(e) =
                    execute_async_transform(transform, &mut normalized, de_service, run_uuid).await
                {
                    // Log error - use "error" level so it's visible in logs UI
                    let error_msg = e.to_string();
                    log::error!(
                        "[workflow] Async transform failed for item {item_uuid} at step {step_idx}: {error_msg}"
                    );
                    if let Err(log_err) = repo
                        .insert_run_log(
                            run_uuid,
                            "error",
                            &format!("Step {step_idx}: Async transform failed"),
                            Some(serde_json::json!({
                                "item_uuid": item_uuid,
                                "step_idx": step_idx,
                                "transform_type": format!("{:?}", transform),
                                "error": error_msg
                            })),
                        )
                        .await
                    {
                        log::error!("[workflow] Failed to insert run log: {log_err}");
                    }
                    // Continue without the async transform results (zero-impact resilience)
                }
            }
        }

        // Step 3: Apply BuildPath transform (now has access to async transform results)
        if matches!(transform, Transform::BuildPath(_)) {
            DslProgram::apply_build_path(step_idx, transform, &mut normalized)?;
        }

        // Step 4: Finalize the step (apply output mapping)
        let (to_def, produced) = program.finalize_step(step_idx, &normalized)?;

        // Step 5: Determine what to pass to the next step
        previous_step_output =
            Some(program.get_next_step_input(step_idx, &normalized, &produced)?);

        results.push((to_def, produced));
    }

    Ok(results)
}

async fn process_outputs(
    processed_outputs: Vec<(ToDef, JsonValue)>,
    payload: &JsonValue,
    item_uuid: Uuid,
    run_uuid: Uuid,
    versioning_disabled: bool,
    dynamic_entity_service: Option<&DynamicEntityService>,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> r_data_core_core::error::Result<bool> {
    let mut entity_ops_ok = true;
    for (to_def, produced) in processed_outputs {
        // Handle Format outputs with Push mode
        let push_ok =
            handle_format_push_output(&to_def, &produced, item_uuid, run_uuid, repo).await?;
        if !push_ok {
            entity_ops_ok = false;
            break;
        }

        // Handle Entity outputs
        if let Some(de_service) = dynamic_entity_service {
            let entity_params = crate::workflow::output_handling::EntityOutputParams {
                produced: produced.clone(),
                payload: payload.clone(),
                item_uuid,
                run_uuid,
                versioning_disabled,
                dynamic_entity_service: de_service,
                repo,
            };
            let entity_ok = handle_entity_output(&to_def, entity_params).await?;
            if !entity_ok {
                entity_ops_ok = false;
                break;
            }
        }
    }

    if entity_ops_ok {
        mark_item_processed(item_uuid, run_uuid, repo).await
    } else {
        // Entity op failed, mark item failed
        log::error!("[workflow] Item {item_uuid} failed: entity operation failed");
        if let Err(e) = repo
            .set_raw_item_status(item_uuid, "failed", Some("entity operation failed"))
            .await
        {
            log::error!("[workflow] Failed to mark item {item_uuid} as failed: {e}");
        }
        Ok(false)
    }
}

async fn mark_item_processed(
    item_uuid: Uuid,
    run_uuid: Uuid,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> r_data_core_core::error::Result<bool> {
    if let Err(e) = repo.set_raw_item_status(item_uuid, "processed", None).await {
        let db_meta = extract_sqlx_meta(&e);
        log::error!("[workflow] Failed to mark item {item_uuid} as processed: {e}");
        if let Err(log_err) = repo
            .insert_run_log(
                run_uuid,
                "error",
                "Failed to mark item processed",
                Some(serde_json::json!({
                    "item_uuid": item_uuid,
                    "attempted_status": "processed",
                    "error": e.to_string(),
                    "db": db_meta
                })),
            )
            .await
        {
            log::error!("[workflow] Failed to insert run log: {log_err}");
        }
        return Ok(false);
    }
    Ok(true)
}

async fn handle_execution_error(
    e: r_data_core_core::error::Error,
    item_uuid: Uuid,
    run_uuid: Uuid,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> r_data_core_core::error::Result<bool> {
    let error_msg = e.to_string();

    // Log to stdout/stderr for visibility
    log::error!("[workflow] Item {item_uuid} failed: {error_msg}");

    // Mark item as error to prevent reprocessing
    if let Err(set_err) = repo
        .set_raw_item_status(item_uuid, "failed", Some(&error_msg))
        .await
    {
        let db_meta = extract_sqlx_meta(&set_err);
        log::error!("[workflow] Failed to mark item {item_uuid} as failed: {set_err}");
        if let Err(log_err) = repo
            .insert_run_log(
                run_uuid,
                "error",
                "Failed to mark item failed",
                Some(serde_json::json!({
                    "item_uuid": item_uuid,
                    "attempted_status": "failed",
                    "error": set_err.to_string(),
                    "db": db_meta
                })),
            )
            .await
        {
            log::error!("[workflow] Failed to insert run log: {log_err}");
        }
    }

    // Log the actual error to workflow_run_logs
    if let Err(log_err) = repo
        .insert_run_log(
            run_uuid,
            "error",
            "Item processing failed",
            Some(serde_json::json!({
                "item_uuid": item_uuid,
                "error": error_msg,
                "error_type": format!("{:?}", e)
            })),
        )
        .await
    {
        log::error!("[workflow] Failed to insert run log: {log_err}");
    }

    Ok(false)
}

fn extract_sqlx_meta(e: &r_data_core_core::error::Error) -> serde_json::Value {
    // Walk the error chain and extract sqlx::Error::Database details if present
    // Fall back to debug formatting of the full chain
    let (code, message) =
        if let r_data_core_core::error::Error::Database(sqlx::Error::Database(db_err)) = e {
            (
                db_err.code().map(|s| s.to_string()),
                Some(db_err.message().to_string()),
            )
        } else {
            // Try to walk the error chain
            let mut code: Option<String> = None;
            let mut message: Option<String> = None;
            let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(e);
            while let Some(err) = cause {
                if let Some(sqlx::Error::Database(db_err)) = err.downcast_ref::<sqlx::Error>() {
                    code = db_err.code().map(|s| s.to_string());
                    message = Some(db_err.message().to_string());
                    break;
                }
                cause = err.source();
            }
            (code, message)
        };

    serde_json::json!({
        "code": code,
        "message": message,
        "chain": format!("{:?}", e),
    })
}
