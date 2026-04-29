use crate::workflow::item_processing::WorkflowItemContext;
use crate::workflow::outbox::enqueue_workflow_push_outbox;
use crate::workflow::outbox::PushDispatchMode;
use r_data_core_workflow::dsl::ToDef;
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub(super) struct WorkflowPushOutputHandler<'a> {
    ctx: &'a WorkflowItemContext<'a>,
}

impl<'a> WorkflowPushOutputHandler<'a> {
    pub(super) const fn new(ctx: &'a WorkflowItemContext<'a>) -> Self {
        Self { ctx }
    }

    pub(super) async fn handle(
        &self,
        to_def: &ToDef,
        produced: &JsonValue,
        workflow_uuid: Uuid,
        step_index: usize,
        item_uuid: Uuid,
        run_uuid: Uuid,
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
            let data_bytes = self
                .serialize_for_push(format, produced, item_uuid, run_uuid)
                .await?;
            match self.ctx.push_dispatch {
                PushDispatchMode::Outbox { repository } => {
                    enqueue_workflow_push_outbox(
                        repository,
                        workflow_uuid,
                        run_uuid,
                        item_uuid,
                        step_index,
                        &destination.destination_type,
                        destination.config.clone(),
                        destination.auth.is_some(),
                        method.as_ref().copied(),
                        format.format_type.as_str(),
                        &data_bytes,
                    )
                    .await?;
                }
                PushDispatchMode::Direct => {
                    let dest_ctx =
                        Self::create_destination_context(destination, method.as_ref().copied())?;
                    let dest_adapter = self
                        .create_destination_adapter(destination, item_uuid, run_uuid)
                        .await?;
                    self.push_data(
                        dest_adapter,
                        &dest_ctx,
                        data_bytes,
                        destination,
                        item_uuid,
                        run_uuid,
                    )
                    .await?;
                }
            }
        }
        Ok(true)
    }

    async fn serialize_for_push(
        &self,
        format: &r_data_core_workflow::dsl::from::FormatConfig,
        produced: &JsonValue,
        item_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<Vec<u8>> {
        let format_handler: Box<dyn r_data_core_workflow::data::adapters::format::FormatHandler> =
            match format.format_type.as_str() {
                "csv" => Box::new(
                    r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler::new(),
                ),
                "json" => Box::new(
                    r_data_core_workflow::data::adapters::format::json::JsonFormatHandler::new(),
                ),
                _ => {
                    self.ctx
                        .repo
                        .insert_run_log(
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
            let _ = self
                .ctx
                .repo
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

        result.map_err(|e| {
            r_data_core_core::error::Error::Unknown(format!("Failed to serialize: {e}"))
        })
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
            .map(|auth_cfg| {
                r_data_core_workflow::data::adapters::auth::create_auth_provider(auth_cfg)
            })
            .transpose()
            .map_err(|e| {
                r_data_core_core::error::Error::Config(format!(
                    "Failed to create auth provider: {e}"
                ))
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
        &self,
        destination: &r_data_core_workflow::dsl::to::DestinationConfig,
        item_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<
        Box<dyn r_data_core_workflow::data::adapters::destination::DataDestination>,
    > {
        if destination.destination_type.as_str() == "uri" {
            Ok(Box::new(
                r_data_core_workflow::data::adapters::destination::uri::UriDestination::new(),
            ))
        } else {
            let _ = self
                .ctx
                .repo
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
        &self,
        dest_adapter: Box<dyn r_data_core_workflow::data::adapters::destination::DataDestination>,
        dest_ctx: &r_data_core_workflow::data::adapters::destination::DestinationContext,
        data_bytes: Vec<u8>,
        destination: &r_data_core_workflow::dsl::to::DestinationConfig,
        item_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<()> {
        use bytes::Bytes;
        let result = dest_adapter.push(dest_ctx, Bytes::from(data_bytes)).await;

        if let Err(ref e) = result {
            let _ = self
                .ctx
                .repo
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
}
