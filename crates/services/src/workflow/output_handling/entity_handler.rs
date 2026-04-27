use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::entity_persistence::{
    create_entity, create_or_update_entity, update_entity, PersistenceContext,
};
use crate::workflow::item_processing::WorkflowItemContext;
use r_data_core_workflow::dsl::path_resolution::build_path_from_fields;
use r_data_core_workflow::dsl::{EntityWriteMode, ToDef};
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub(super) struct WorkflowEntityOutputHandler<'a> {
    ctx: &'a WorkflowItemContext<'a>,
}

struct EntityOperationArgs<'a> {
    mode: &'a EntityWriteMode,
    produced: &'a JsonValue,
    entity_definition: &'a str,
    path: Option<String>,
    run_uuid: Uuid,
    dynamic_entity_service: &'a DynamicEntityService,
    ctx: &'a PersistenceContext,
}

impl<'a> WorkflowEntityOutputHandler<'a> {
    pub(super) const fn new(ctx: &'a WorkflowItemContext<'a>) -> Self {
        Self { ctx }
    }

    pub(super) async fn handle(
        &self,
        to_def: &ToDef,
        produced: &JsonValue,
        payload: &JsonValue,
        item_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        let ToDef::Entity {
            entity_definition,
            path,
            mode,
            identify: _,
            update_key,
            mapping: _,
        } = to_def
        else {
            return Ok(true);
        };

        let Some(dynamic_entity_service) = self.ctx.dynamic_entity_service else {
            return Ok(true);
        };

        let resolved_path = path
            .as_ref()
            .map(|raw_path| Self::resolve_path_template(raw_path, produced));
        let produced_for_update =
            Self::prepare_produced_for_update(mode, produced, payload, update_key.as_ref());

        let ctx = PersistenceContext {
            entity_type: entity_definition.clone(),
            produced: produced_for_update,
            path: resolved_path.clone(),
            run_uuid,
            update_key: update_key.clone(),
            skip_versioning: self.ctx.versioning_disabled,
        };

        let result = self
            .execute_entity_operation(EntityOperationArgs {
                mode,
                produced,
                entity_definition,
                path: resolved_path,
                run_uuid,
                dynamic_entity_service,
                ctx: &ctx,
            })
            .await;

        let success = self
            .handle_entity_result(result, mode, entity_definition, item_uuid, run_uuid)
            .await;
        Ok(success)
    }

    fn resolve_path_template(path: &str, produced: &JsonValue) -> String {
        if !path.contains('{') {
            return path.to_string();
        }

        match build_path_from_fields::<std::collections::hash_map::RandomState>(
            path, produced, None, None,
        ) {
            Ok(resolved) => resolved,
            Err(e) => {
                log::warn!(
                    "Failed to resolve path template '{path}' from produced data: {e}. Using path as-is."
                );
                path.to_string()
            }
        }
    }

    fn prepare_produced_for_update(
        mode: &EntityWriteMode,
        produced: &JsonValue,
        payload: &JsonValue,
        update_key: Option<&String>,
    ) -> JsonValue {
        if matches!(
            mode,
            EntityWriteMode::Update | EntityWriteMode::CreateOrUpdate
        ) {
            let mut merged = produced.clone();
            if let (Some(merged_obj), Some(payload_obj)) =
                (merged.as_object_mut(), payload.as_object())
            {
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

    async fn execute_entity_operation(
        &self,
        args: EntityOperationArgs<'_>,
    ) -> r_data_core_core::error::Result<()> {
        match args.mode {
            EntityWriteMode::Create => {
                let create_ctx = PersistenceContext {
                    entity_type: args.entity_definition.to_string(),
                    produced: args.produced.clone(),
                    path: args.path,
                    run_uuid: args.run_uuid,
                    update_key: None,
                    skip_versioning: self.ctx.versioning_disabled,
                };
                create_entity(args.dynamic_entity_service, &create_ctx).await
            }
            EntityWriteMode::Update => update_entity(args.dynamic_entity_service, args.ctx).await,
            EntityWriteMode::CreateOrUpdate => {
                create_or_update_entity(args.dynamic_entity_service, args.ctx).await
            }
        }
    }

    async fn handle_entity_result(
        &self,
        result: r_data_core_core::error::Result<()>,
        mode: &EntityWriteMode,
        entity_definition: &str,
        item_uuid: Uuid,
        run_uuid: Uuid,
    ) -> bool {
        if let Err(e) = result {
            let operation = match mode {
                EntityWriteMode::Create => "create",
                EntityWriteMode::Update => "update",
                EntityWriteMode::CreateOrUpdate => "create_or_update",
            };
            let error_msg = e.to_string();

            log::error!(
                "[workflow] Entity {operation} failed for item {item_uuid}, type '{entity_definition}': {error_msg}"
            );

            if let Err(log_err) = self
                .ctx
                .repo
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
}
