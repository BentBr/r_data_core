#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::workflow::entity_persistence::{
    create_entity, create_or_update_entity, update_entity, PersistenceContext,
};
use r_data_core_persistence::WorkflowRepositoryTrait;
use r_data_core_workflow::dsl::ToDef;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

/// Handle Format outputs with Push mode
///
/// # Errors
/// Returns an error if serialization, authentication, or push fails
pub async fn handle_format_push_output(
    to_def: &ToDef,
    produced: &JsonValue,
    item_uuid: Uuid,
    run_uuid: Uuid,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> r_data_core_core::error::Result<bool> {
    if let ToDef::Format {
        output:
            r_data_core_workflow::dsl::OutputMode::Push {
                ref destination,
                ref method,
            },
        ref format,
        ..
    } = to_def
    {
        let data_bytes = serialize_for_push(format, produced, item_uuid, run_uuid, repo).await?;
        let dest_ctx = create_destination_context(destination, method.as_ref().copied())?;
        let dest_adapter =
            create_destination_adapter(destination, item_uuid, run_uuid, repo).await?;
        push_data(
            dest_adapter,
            &dest_ctx,
            data_bytes,
            destination,
            item_uuid,
            run_uuid,
            repo,
        )
        .await?;
    }
    Ok(true) // Not a push output, nothing to handle
}

async fn serialize_for_push(
    format: &r_data_core_workflow::dsl::from::FormatConfig,
    produced: &JsonValue,
    item_uuid: Uuid,
    run_uuid: Uuid,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> r_data_core_core::error::Result<Vec<u8>> {
    let format_handler: Box<dyn r_data_core_workflow::data::adapters::format::FormatHandler> =
        match format.format_type.as_str() {
            "csv" => {
                Box::new(r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler::new())
            }
            "json" => Box::new(
                r_data_core_workflow::data::adapters::format::json::JsonFormatHandler::new(),
            ),
            _ => {
                repo.insert_run_log(
                    run_uuid,
                    "error",
                    "Unsupported format for push",
                    Some(serde_json::json!({
                        "item_uuid": item_uuid,
                        "format_type": format.format_type
                    })),
                )
                .await
                .ok();
                return Err(r_data_core_core::error::Error::Validation(
                    "Unsupported format for push".to_string(),
                ));
            }
        };

    let result = format_handler
        .serialize(std::slice::from_ref(produced), &format.options)
        .map(|bytes| bytes.to_vec());

    if let Err(ref e) = result {
        let _ = repo
            .insert_run_log(
                run_uuid,
                "error",
                "Failed to serialize data for push",
                Some(serde_json::json!({
                    "item_uuid": item_uuid,
                    "error": e.to_string()
                })),
            )
            .await;
    }

    result.map_err(|e| r_data_core_core::error::Error::Unknown(format!("Failed to serialize: {e}")))
}

fn create_destination_context(
    destination: &r_data_core_workflow::dsl::to::DestinationConfig,
    method: Option<r_data_core_workflow::data::adapters::destination::HttpMethod>,
) -> r_data_core_core::error::Result<
    r_data_core_workflow::data::adapters::destination::DestinationContext,
> {
    let auth_provider = destination
        .auth
        .as_ref()
        .map(|auth_cfg| r_data_core_workflow::data::adapters::auth::create_auth_provider(auth_cfg))
        .transpose()
        .map_err(|e| {
            r_data_core_core::error::Error::Config(format!("Failed to create auth provider: {e}"))
        })?;

    Ok(
        r_data_core_workflow::data::adapters::destination::DestinationContext {
            auth: auth_provider,
            method: method.as_ref().copied(),
            config: destination.config.clone(),
        },
    )
}

async fn create_destination_adapter(
    destination: &r_data_core_workflow::dsl::to::DestinationConfig,
    item_uuid: Uuid,
    run_uuid: Uuid,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> r_data_core_core::error::Result<
    Box<dyn r_data_core_workflow::data::adapters::destination::DataDestination>,
> {
    if destination.destination_type.as_str() == "uri" {
        Ok(Box::new(
            r_data_core_workflow::data::adapters::destination::uri::UriDestination::new(),
        ))
    } else {
        let _ = repo
            .insert_run_log(
                run_uuid,
                "error",
                "Unsupported destination type",
                Some(serde_json::json!({
                    "item_uuid": item_uuid,
                    "destination_type": destination.destination_type
                })),
            )
            .await;
        Err(r_data_core_core::error::Error::Validation(
            "Unsupported destination type".to_string(),
        ))
    }
}

async fn push_data(
    dest_adapter: Box<dyn r_data_core_workflow::data::adapters::destination::DataDestination>,
    dest_ctx: &r_data_core_workflow::data::adapters::destination::DestinationContext,
    data_bytes: Vec<u8>,
    destination: &r_data_core_workflow::dsl::to::DestinationConfig,
    item_uuid: Uuid,
    run_uuid: Uuid,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> r_data_core_core::error::Result<()> {
    use bytes::Bytes;
    let result = dest_adapter.push(dest_ctx, Bytes::from(data_bytes)).await;

    if let Err(ref e) = result {
        let _ = repo
            .insert_run_log(
                run_uuid,
                "error",
                "Failed to push data to destination",
                Some(serde_json::json!({
                    "item_uuid": item_uuid,
                    "destination_type": destination.destination_type,
                    "error": e.to_string()
                })),
            )
            .await;
    }

    result.map_err(|e| r_data_core_core::error::Error::Api(format!("Failed to push: {e}")))
}

/// Parameters for handling entity output
pub struct EntityOutputParams<'a> {
    pub produced: JsonValue,
    pub payload: JsonValue,
    pub item_uuid: Uuid,
    pub run_uuid: Uuid,
    pub versioning_disabled: bool,
    pub dynamic_entity_service: &'a crate::dynamic_entity::DynamicEntityService,
    pub repo: &'a Arc<dyn WorkflowRepositoryTrait>,
}

/// Handle Entity outputs
///
/// # Errors
/// Returns an error if entity creation/update fails
pub async fn handle_entity_output(
    to_def: &ToDef,
    params: EntityOutputParams<'_>,
) -> r_data_core_core::error::Result<bool> {
    if let ToDef::Entity {
        entity_definition,
        path,
        mode,
        identify: _,
        update_key,
        mapping: _,
    } = to_def
    {
        let produced_for_update = prepare_produced_for_update(
            mode,
            &params.produced,
            &params.payload,
            update_key.as_ref(),
        );

        let ctx = PersistenceContext {
            entity_type: entity_definition.clone(),
            produced: produced_for_update.clone(),
            path: Some(path.clone()),
            run_uuid: params.run_uuid,
            update_key: update_key.clone(),
            skip_versioning: params.versioning_disabled,
        };

        let op_params = EntityOperationParams {
            mode: mode.clone(),
            produced: params.produced.clone(),
            ctx: &ctx,
            entity_definition: entity_definition.to_owned(),
            path: path.to_owned(),
            run_uuid: params.run_uuid,
            versioning_disabled: params.versioning_disabled,
        };
        let result = execute_entity_operation(op_params, params.dynamic_entity_service).await;

        let success = handle_entity_result(
            result,
            mode,
            entity_definition,
            params.item_uuid,
            params.run_uuid,
            params.repo,
        )
        .await;
        Ok(success)
    } else {
        Ok(true) // Not an entity output, nothing to handle
    }
}

fn prepare_produced_for_update(
    mode: &r_data_core_workflow::dsl::EntityWriteMode,
    produced: &JsonValue,
    payload: &JsonValue,
    update_key: Option<&String>,
) -> JsonValue {
    if matches!(
        mode,
        r_data_core_workflow::dsl::EntityWriteMode::Update
            | r_data_core_workflow::dsl::EntityWriteMode::CreateOrUpdate
    ) {
        let mut merged = produced.clone();
        if let (Some(merged_obj), Some(payload_obj)) = (merged.as_object_mut(), payload.as_object())
        {
            // Merge payload fields into produced (payload takes precedence for update_key)
            for (k, v) in payload_obj {
                if k == "entity_key" || update_key.is_some_and(|uk| k == uk) {
                    merged_obj.insert(k.clone(), v.clone());
                }
            }
        }
        merged
    } else {
        produced.clone()
    }
}

struct EntityOperationParams<'a> {
    mode: r_data_core_workflow::dsl::EntityWriteMode,
    produced: JsonValue,
    ctx: &'a PersistenceContext,
    entity_definition: String,
    path: String,
    run_uuid: Uuid,
    versioning_disabled: bool,
}

async fn execute_entity_operation(
    params: EntityOperationParams<'_>,
    dynamic_entity_service: &crate::dynamic_entity::DynamicEntityService,
) -> r_data_core_core::error::Result<()> {
    match params.mode {
        r_data_core_workflow::dsl::EntityWriteMode::Create => {
            let create_ctx = PersistenceContext {
                entity_type: params.entity_definition.clone(),
                produced: params.produced.clone(),
                path: Some(params.path.clone()),
                run_uuid: params.run_uuid,
                update_key: None,
                skip_versioning: params.versioning_disabled,
            };
            create_entity(dynamic_entity_service, &create_ctx).await
        }
        r_data_core_workflow::dsl::EntityWriteMode::Update => {
            update_entity(dynamic_entity_service, params.ctx).await
        }
        r_data_core_workflow::dsl::EntityWriteMode::CreateOrUpdate => {
            create_or_update_entity(dynamic_entity_service, params.ctx).await
        }
    }
}

async fn handle_entity_result(
    result: r_data_core_core::error::Result<()>,
    mode: &r_data_core_workflow::dsl::EntityWriteMode,
    entity_definition: &str,
    item_uuid: Uuid,
    run_uuid: Uuid,
    repo: &Arc<dyn WorkflowRepositoryTrait>,
) -> bool {
    if let Err(e) = result {
        let operation = match mode {
            r_data_core_workflow::dsl::EntityWriteMode::Create => "create",
            r_data_core_workflow::dsl::EntityWriteMode::Update => "update",
            r_data_core_workflow::dsl::EntityWriteMode::CreateOrUpdate => "create_or_update",
        };
        let error_msg = e.to_string();

        // Log to stdout/stderr for visibility
        log::error!(
            "[workflow] Entity {operation} failed for item {item_uuid}, type '{entity_definition}': {error_msg}"
        );

        // Log to database for workflow run history
        if let Err(log_err) = repo
            .insert_run_log(
                run_uuid,
                "error",
                &format!("Entity {operation} failed for '{entity_definition}'"),
                Some(serde_json::json!({
                    "item_uuid": item_uuid,
                    "entity_type": entity_definition,
                    "mode": format!("{:?}", mode),
                    "error": error_msg
                })),
            )
            .await
        {
            log::error!("[workflow] Failed to insert run log: {log_err}");
        }
        return false;
    }
    true
}
