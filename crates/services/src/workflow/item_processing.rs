#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::output_handling::{handle_entity_output, handle_format_push_output};
use r_data_core_persistence::WorkflowRepositoryTrait;
use r_data_core_workflow::dsl::DslProgram;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

/// Process a single item through the workflow DSL
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
    match program.execute(payload) {
        Ok(outputs) => {
            let mut entity_ops_ok = true;
            for (to_def, produced) in outputs {
                // Handle Format outputs with Push mode
                let push_ok =
                    handle_format_push_output(&to_def, &produced, item_uuid, run_uuid, repo)
                        .await?;
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
                // Mark processed
                if let Err(e) = repo.set_raw_item_status(item_uuid, "processed", None).await {
                    let db_meta = extract_sqlx_meta(&e);
                    let _ = repo
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
                        .await;
                    return Ok(false);
                }
                Ok(true)
            } else {
                // Entity op failed, mark item failed
                let _ = repo
                    .set_raw_item_status(item_uuid, "failed", Some("entity operation failed"))
                    .await;
                Ok(false)
            }
        }
        Err(e) => {
            // Mark item as error to prevent reprocessing
            if let Err(set_err) = repo
                .set_raw_item_status(item_uuid, "failed", Some(&e.to_string()))
                .await
            {
                let db_meta = extract_sqlx_meta(&set_err);
                let _ = repo
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
                    .await;
            }
            let _ = repo
                .insert_run_log(
                    run_uuid,
                    "error",
                    "DSL execute failed for item; item marked as error",
                    Some(serde_json::json!({ "item_uuid": item_uuid, "error": e.to_string() })),
                )
                .await;
            Ok(false)
        }
    }
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
            // Try to walk the error chain for Anyhow errors
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
