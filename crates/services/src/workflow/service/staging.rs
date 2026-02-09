use futures::StreamExt;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

use super::WorkflowService;

impl WorkflowService {
    fn infer_input_type(cfg: &serde_json::Value) -> Option<String> {
        // Required structure: { "input": { "type": "csv" | "ndjson", "format": {...}, "source": {...} } }
        cfg.pointer("/input/type")
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
    }

    fn csv_format_from_config(cfg: &serde_json::Value) -> serde_json::Value {
        cfg.pointer("/input/format").cloned().unwrap_or_else(
            || serde_json::json!({ "has_header": true, "delimiter": ",", "quote": "\"" }),
        )
    }

    /// Stage raw items for processing
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn stage_raw_items(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        payloads: Vec<serde_json::Value>,
    ) -> r_data_core_core::error::Result<i64> {
        self.repo
            .insert_raw_items(workflow_uuid, run_uuid, payloads)
            .await
    }

    /// Handle a CSV upload for a run-now execution:
    /// - creates a run (queued)
    /// - parses CSV (expects headers)
    /// - stages rows as raw items
    /// - writes a staging log
    ///
    /// # Errors
    /// Returns an error if parsing fails or database operation fails
    pub async fn run_now_upload_csv(
        &self,
        workflow_uuid: Uuid,
        bytes: &[u8],
    ) -> r_data_core_core::error::Result<(Uuid, i64)> {
        let run_uuid = self.enqueue_run(workflow_uuid).await?;

        // Read workflow config for input options
        let wf = self.repo.get_by_uuid(workflow_uuid).await?.ok_or_else(|| {
            r_data_core_core::error::Error::NotFound("Workflow not found".to_string())
        })?;
        let input_type = Self::infer_input_type(&wf.config).unwrap_or_else(|| "csv".to_string());
        let inline = String::from_utf8_lossy(bytes).to_string();
        let payloads = match input_type.as_str() {
            "csv" => {
                use r_data_core_workflow::data::adapters::format::FormatHandler;
                let format_cfg = Self::csv_format_from_config(&wf.config);
                r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler::new()
                    .parse(inline.as_bytes(), &format_cfg)
                    .map_err(|e| {
                        r_data_core_core::error::Error::Validation(format!(
                            "Failed to parse CSV data: {e}"
                        ))
                    })?
            }
            "ndjson" => inline
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(serde_json::from_str::<serde_json::Value>)
                .collect::<Result<Vec<_>, _>>()?,
            other => {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "Unsupported input type for upload: {other}"
                )))
            }
        };

        if payloads.is_empty() {
            // Nothing to stage
            self.repo
                .insert_run_log(run_uuid, "warn", "CSV upload contained no data rows", None)
                .await
                .ok();
            return Ok((run_uuid, 0));
        }

        let staged = self
            .stage_raw_items(workflow_uuid, run_uuid, payloads)
            .await?;
        let _ = self
            .repo
            .insert_run_log(
                run_uuid,
                "info",
                "Upload staged",
                Some(serde_json::json!({ "staged_items": staged, "input_type": input_type })),
            )
            .await;
        Ok((run_uuid, staged))
    }

    /// Upload bytes (CSV/JSON) and trigger workflow run synchronously
    ///
    /// # Errors
    /// Returns an error if parsing fails or database operation fails
    pub async fn run_now_upload_bytes(
        &self,
        workflow_uuid: Uuid,
        bytes: &[u8],
    ) -> r_data_core_core::error::Result<(Uuid, i64)> {
        let run_uuid = self.enqueue_run(workflow_uuid).await?;

        // Read workflow config for input options
        let wf = self.repo.get_by_uuid(workflow_uuid).await?.ok_or_else(|| {
            r_data_core_core::error::Error::NotFound("Workflow not found".to_string())
        })?;

        // Try to infer format from DSL
        let program =
            r_data_core_workflow::dsl::DslProgram::from_config(&wf.config).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!(
                    "Invalid workflow DSL configuration: {e}"
                ))
            })?;
        let format_type = program
            .steps
            .first()
            .and_then(|step| {
                if let r_data_core_workflow::dsl::FromDef::Format { format, .. } = &step.from {
                    Some(format.format_type.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "csv".to_string());

        let payloads = match format_type.as_str() {
            "csv" => {
                let format_cfg = program
                    .steps
                    .first()
                    .and_then(|step| {
                        if let r_data_core_workflow::dsl::FromDef::Format { format, .. } =
                            &step.from
                        {
                            Some(format.options.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| serde_json::json!({}));
                {
                    use r_data_core_workflow::data::adapters::format::FormatHandler;
                    r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler::new()
                        .parse(bytes, &format_cfg)
                        .map_err(|e| {
                            r_data_core_core::error::Error::Validation(format!(
                                "Failed to parse CSV data: {e}"
                            ))
                        })?
                }
            }
            "json" => {
                let format_cfg = program
                    .steps
                    .first()
                    .and_then(|step| {
                        if let r_data_core_workflow::dsl::FromDef::Format { format, .. } =
                            &step.from
                        {
                            Some(format.options.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| serde_json::json!({}));
                {
                    use r_data_core_workflow::data::adapters::format::FormatHandler;
                    r_data_core_workflow::data::adapters::format::json::JsonFormatHandler::new()
                        .parse(bytes, &format_cfg)
                        .map_err(|e| {
                            r_data_core_core::error::Error::Validation(format!(
                                "Failed to parse JSON data: {e}"
                            ))
                        })?
                }
            }
            other => {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "Unsupported input type for upload: {other}"
                )))
            }
        };

        if payloads.is_empty() {
            self.repo
                .insert_run_log(run_uuid, "warn", "Upload contained no data rows", None)
                .await
                .ok();
            return Ok((run_uuid, 0));
        }

        let staged = self
            .stage_raw_items(workflow_uuid, run_uuid, payloads)
            .await?;
        let _ = self
            .repo
            .insert_run_log(
                run_uuid,
                "info",
                "Upload staged",
                Some(serde_json::json!({ "staged_items": staged, "input_type": format_type })),
            )
            .await;
        Ok((run_uuid, staged))
    }

    /// Fetch from configured source (URI) and stage items using the appropriate adapter (csv or ndjson)
    ///
    /// # Errors
    /// Returns an error if DSL parsing fails, fetch fails, or staging fails
    pub async fn fetch_and_stage_from_config(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<i64> {
        let wf = self.repo.get_by_uuid(workflow_uuid).await?.ok_or_else(|| {
            r_data_core_core::error::Error::NotFound("Workflow not found".to_string())
        })?;

        // Parse DSL program to get FromDef steps
        let program =
            r_data_core_workflow::dsl::DslProgram::from_config(&wf.config).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!(
                    "Failed to parse DSL for fetch: {e}"
                ))
            })?;

        // Find Format-based and Entity-based FromDef steps that need fetching
        let mut total_staged = 0_i64;
        for step in &program.steps {
            if let r_data_core_workflow::dsl::FromDef::Entity {
                entity_definition,
                filter,
                ..
            } = &step.from
            {
                let staged = self
                    .handle_entity_source(
                        entity_definition,
                        filter.as_ref(),
                        workflow_uuid,
                        run_uuid,
                    )
                    .await?;
                total_staged += staged;
                continue;
            }

            if let r_data_core_workflow::dsl::FromDef::Format { source, format, .. } = &step.from {
                if source.source_type == "api" {
                    continue;
                }
                let staged = self
                    .handle_format_source(source, format, workflow_uuid, run_uuid)
                    .await?;
                total_staged += staged;
            }
            // Skip trigger - it has no data to fetch (no-op, loop continues naturally)
        }

        Ok(total_staged)
    }

    async fn handle_entity_source(
        &self,
        entity_definition: &str,
        filter: Option<&r_data_core_workflow::dsl::EntityFilter>,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<i64> {
        let Some(entity_service) = &self.dynamic_entity_service else {
            return Err(r_data_core_core::error::Error::Api(
                "Entity service not available for entity source workflow".to_string(),
            ));
        };

        let (filter_map, operators_map) = build_filter_maps(filter);
        let (filters_opt, operators_opt) = if filter_map.is_empty() {
            (None, None)
        } else {
            (Some(filter_map), Some(operators_map))
        };
        let entities = entity_service
            .filter_entities_with_operators(
                entity_definition,
                10000, // Large limit for exports
                0,
                filters_opt,
                operators_opt,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| {
                r_data_core_core::error::Error::Entity(format!("Failed to fetch entities: {e}"))
            })?;

        let entity_count = i64::try_from(entities.len()).unwrap_or(0);

        let payloads: Vec<JsonValue> = entities
            .into_iter()
            .map(|entity| {
                serde_json::to_value(&entity.field_data).unwrap_or_else(|_| serde_json::json!({}))
            })
            .collect();

        let staged = self
            .stage_raw_items(workflow_uuid, run_uuid, payloads)
            .await?;

        self.repo
            .insert_run_log(
                run_uuid,
                "info",
                &format!("Fetched {entity_count} entities from {entity_definition}"),
                Some(serde_json::json!({
                    "staged_items": staged,
                    "entity_count": entity_count,
                    "entity_definition": entity_definition,
                    "filter_field": filter.map(|f| f.field.clone()),
                    "filter_operator": filter.map(|f| f.operator.clone()),
                    "filter_value": filter.map(|f| f.value.clone())
                })),
            )
            .await?;

        Ok(staged)
    }

    async fn handle_format_source(
        &self,
        source: &r_data_core_workflow::dsl::from::SourceConfig,
        format: &r_data_core_workflow::dsl::from::FormatConfig,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<i64> {
        let auth_provider = source
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

        let source_ctx = r_data_core_workflow::data::adapters::source::SourceContext {
            auth: auth_provider,
            config: source.config.clone(),
        };

        let source_adapter: Box<dyn r_data_core_workflow::data::adapters::source::DataSource> =
            match source.source_type.as_str() {
                "uri" => {
                    Box::new(r_data_core_workflow::data::adapters::source::uri::UriSource::new())
                }
                _ => {
                    return Err(r_data_core_core::error::Error::Validation(format!(
                        "Unsupported source type: {}",
                        source.source_type
                    )));
                }
            };

        let mut stream = source_adapter.fetch(&source_ctx).await.map_err(|e| {
            r_data_core_core::error::Error::Api(format!("Failed to fetch data from source: {e}"))
        })?;
        let mut all_data = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                r_data_core_core::error::Error::Api(format!("Failed to read data chunk: {e}"))
            })?;
            all_data.extend_from_slice(&chunk);
        }

        let format_handler: Box<dyn r_data_core_workflow::data::adapters::format::FormatHandler> =
            match format.format_type.as_str() {
                "csv" => Box::new(
                    r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler::new(),
                ),
                "json" => Box::new(
                    r_data_core_workflow::data::adapters::format::json::JsonFormatHandler::new(),
                ),
                _ => {
                    return Err(r_data_core_core::error::Error::Validation(format!(
                        "Unsupported format type: {}",
                        format.format_type
                    )));
                }
            };

        let payloads = format_handler
            .parse(&all_data, &format.options)
            .map_err(|e| {
                r_data_core_core::error::Error::Validation(format!(
                    "Failed to parse data format: {e}"
                ))
            })?;
        let staged = self
            .stage_raw_items(workflow_uuid, run_uuid, payloads)
            .await?;

        let _ = self
            .repo
            .insert_run_log(
                run_uuid,
                "info",
                "Fetched and staged",
                Some(serde_json::json!({
                    "staged_items": staged,
                    "source_type": source.source_type,
                    "format_type": format.format_type
                })),
            )
            .await;

        Ok(staged)
    }
}

fn build_filter_maps(
    filter: Option<&r_data_core_workflow::dsl::EntityFilter>,
) -> (HashMap<String, JsonValue>, HashMap<String, String>) {
    let mut filter_map = HashMap::new();
    let mut operators_map = HashMap::new();

    if let Some(filter) = filter {
        let filter_value = if filter.operator == "IN" || filter.operator == "NOT IN" {
            match serde_json::from_str::<JsonValue>(&filter.value) {
                Ok(JsonValue::Array(_)) => serde_json::from_str(&filter.value)
                    .unwrap_or_else(|_| serde_json::json!([filter.value])),
                _ => serde_json::json!([filter.value]),
            }
        } else {
            JsonValue::String(filter.value.clone())
        };
        filter_map.insert(filter.field.clone(), filter_value);

        operators_map.insert(filter.field.clone(), filter.operator.clone());
    }

    (filter_map, operators_map)
}
