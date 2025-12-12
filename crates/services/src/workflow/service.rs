use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::item_processing::process_single_item;
use cron::Schedule;
use futures::StreamExt;
use r_data_core_persistence::WorkflowRepositoryTrait;
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core_workflow::data::Workflow;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub struct WorkflowService {
    repo: Arc<dyn WorkflowRepositoryTrait>,
    dynamic_entity_service: Option<Arc<DynamicEntityService>>,
}

impl WorkflowService {
    pub fn new(repo: Arc<dyn WorkflowRepositoryTrait>) -> Self {
        Self {
            repo,
            dynamic_entity_service: None,
        }
    }

    pub fn new_with_entities(
        repo: Arc<dyn WorkflowRepositoryTrait>,
        dynamic_entity_service: Arc<DynamicEntityService>,
    ) -> Self {
        Self {
            repo,
            dynamic_entity_service: Some(dynamic_entity_service),
        }
    }

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

    /// List all workflows
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list(&self) -> anyhow::Result<Vec<Workflow>> {
        self.repo.list_all().await
    }

    /// Get a workflow by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get(&self, uuid: Uuid) -> anyhow::Result<Option<Workflow>> {
        self.repo.get_by_uuid(uuid).await
    }

    /// Create a new workflow
    ///
    /// # Errors
    /// Returns an error if validation fails or database operation fails
    pub async fn create(
        &self,
        req: &CreateWorkflowRequest,
        created_by: Uuid,
    ) -> anyhow::Result<Uuid> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| anyhow::anyhow!("Invalid cron schedule: {e}"))?;
        }
        // Strict DSL: parse and validate
        let program = r_data_core_workflow::dsl::DslProgram::from_config(&req.config)?;
        program.validate()?;
        self.repo.create(req, created_by).await
    }

    /// Update an existing workflow
    ///
    /// # Errors
    /// Returns an error if validation fails or database operation fails
    pub async fn update(
        &self,
        uuid: Uuid,
        req: &UpdateWorkflowRequest,
        updated_by: Uuid,
    ) -> anyhow::Result<()> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| anyhow::anyhow!("Invalid cron schedule: {e}"))?;
        }
        // Strict DSL: parse and validate
        let program = r_data_core_workflow::dsl::DslProgram::from_config(&req.config)?;
        program.validate()?;
        self.repo.update(uuid, req, updated_by).await
    }

    /// Delete a workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn delete(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.repo.delete(uuid).await
    }

    /// List workflows with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> anyhow::Result<(Vec<Workflow>, i64)> {
        let (items, total) = tokio::try_join!(
            self.repo.list_paginated(limit, offset, sort_by, sort_order),
            self.repo.count_all()
        )?;
        Ok((items, total))
    }

    /// List workflows with query validation
    ///
    /// This method validates the query parameters and returns validated parameters along with workflows.
    ///
    /// # Arguments
    /// * `params` - The query parameters
    /// * `field_validator` - The `FieldValidator` instance (required for validation)
    ///
    /// # Returns
    /// A tuple of ((workflows, total), `validated_query`) where `validated_query` contains pagination metadata
    ///
    /// # Errors
    /// Returns an error if validation fails or database query fails
    pub async fn list_paginated_with_query(
        &self,
        params: &crate::query_validation::ListQueryParams,
        field_validator: &crate::query_validation::FieldValidator,
    ) -> anyhow::Result<(
        (Vec<Workflow>, i64),
        crate::query_validation::ValidatedListQuery,
    )> {
        use crate::query_validation::validate_list_query;

        let validated =
            validate_list_query(params, "workflows", field_validator, 20, 100, true, &[])
                .await
                .map_err(|e| anyhow::anyhow!("Query validation failed: {e}"))?;

        let (items, total) = tokio::try_join!(
            self.repo.list_paginated(
                validated.limit,
                validated.offset,
                validated.sort_by.clone(),
                validated.sort_order.clone(),
            ),
            self.repo.count_all()
        )?;

        Ok(((items, total), validated))
    }

    /// List runs for a workflow with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_runs_paginated(
        &self,
        workflow_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(
        Vec<(
            Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
        )>,
        i64,
    )> {
        self.repo
            .list_runs_paginated(workflow_uuid, limit, offset)
            .await
    }

    /// List run logs with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_run_logs_paginated(
        &self,
        run_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(
        Vec<(Uuid, String, String, String, Option<serde_json::Value>)>,
        i64,
    )> {
        self.repo
            .list_run_logs_paginated(run_uuid, limit, offset)
            .await
    }

    /// Check if a run exists
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn run_exists(&self, run_uuid: Uuid) -> anyhow::Result<bool> {
        self.repo.run_exists(run_uuid).await
    }

    /// List all runs with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_all_runs_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(
        Vec<(
            Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
        )>,
        i64,
    )> {
        self.repo.list_all_runs_paginated(limit, offset).await
    }

    /// Enqueue a workflow run
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn enqueue_run(&self, workflow_uuid: Uuid) -> anyhow::Result<Uuid> {
        let trigger_id = Uuid::now_v7();
        let run_uuid = self
            .repo
            .insert_run_queued(workflow_uuid, trigger_id)
            .await?;
        // Optional: write an initial log entry
        let _ = self
            .repo
            .insert_run_log(
                run_uuid,
                "info",
                "Run enqueued",
                Some(serde_json::json!({ "trigger": trigger_id.to_string() })),
            )
            .await;
        Ok(run_uuid)
    }

    /// Mark a run as running
    ///
    /// # Errors
    /// Returns an error if the database update fails
    pub async fn mark_run_running(&self, run_uuid: Uuid) -> anyhow::Result<()> {
        self.repo.mark_run_running(run_uuid).await
    }

    /// Mark a run as succeeded with processed/failed counts
    ///
    /// # Errors
    /// Returns an error if the database update fails
    pub async fn mark_run_success(
        &self,
        run_uuid: Uuid,
        processed: i64,
        failed: i64,
    ) -> anyhow::Result<()> {
        self.repo
            .mark_run_success(run_uuid, processed, failed)
            .await
    }

    /// Mark a run as failed with error message
    ///
    /// # Errors
    /// Returns an error if the database update fails
    pub async fn mark_run_failure(&self, run_uuid: Uuid, message: &str) -> anyhow::Result<()> {
        self.repo.mark_run_failure(run_uuid, message).await
    }

    /// Insert a log entry for a run
    ///
    /// # Errors
    /// Returns an error if the database insert fails
    pub async fn insert_run_log(
        &self,
        run_uuid: Uuid,
        level: &str,
        message: &str,
        meta: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        self.repo
            .insert_run_log(run_uuid, level, message, meta)
            .await
    }

    /// Get run status (for async polling)
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_run_status(&self, run_uuid: Uuid) -> anyhow::Result<Option<String>> {
        self.repo.get_run_status(run_uuid).await
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
    ) -> anyhow::Result<i64> {
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
    ) -> anyhow::Result<(Uuid, i64)> {
        let run_uuid = self.enqueue_run(workflow_uuid).await?;

        // Read workflow config for input options
        let wf = self
            .repo
            .get_by_uuid(workflow_uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;
        let input_type = Self::infer_input_type(&wf.config).unwrap_or_else(|| "csv".to_string());
        let inline = String::from_utf8_lossy(bytes).to_string();
        let payloads = match input_type.as_str() {
            "csv" => {
                use r_data_core_workflow::data::adapters::format::FormatHandler;
                let format_cfg = Self::csv_format_from_config(&wf.config);
                r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler::new()
                    .parse(inline.as_bytes(), &format_cfg)?
            }
            "ndjson" => inline
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(serde_json::from_str::<serde_json::Value>)
                .collect::<Result<Vec<_>, _>>()?,
            other => {
                return Err(anyhow::anyhow!(format!(
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
    ) -> anyhow::Result<(Uuid, i64)> {
        let run_uuid = self.enqueue_run(workflow_uuid).await?;

        // Read workflow config for input options
        let wf = self
            .repo
            .get_by_uuid(workflow_uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        // Try to infer format from DSL
        let program = r_data_core_workflow::dsl::DslProgram::from_config(&wf.config)?;
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
                        .parse(bytes, &format_cfg)?
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
                        .parse(bytes, &format_cfg)?
                }
            }
            other => {
                return Err(anyhow::anyhow!(format!(
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
    ) -> anyhow::Result<i64> {
        let wf = self
            .repo
            .get_by_uuid(workflow_uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        // Parse DSL program to get FromDef steps
        let program = r_data_core_workflow::dsl::DslProgram::from_config(&wf.config)
            .map_err(|e| anyhow::anyhow!("Failed to parse DSL for fetch: {e}"))?;

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
        }

        Ok(total_staged)
    }

    async fn handle_entity_source(
        &self,
        entity_definition: &str,
        filter: Option<&r_data_core_workflow::dsl::EntityFilter>,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> anyhow::Result<i64> {
        let Some(entity_service) = &self.dynamic_entity_service else {
            return Err(anyhow::anyhow!(
                "Entity service not available for entity source workflow"
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
            .map_err(|e| anyhow::anyhow!("Failed to fetch entities: {e}"))?;

        #[allow(clippy::cast_possible_wrap)]
        let entity_count = entities.len() as i64;

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
    ) -> anyhow::Result<i64> {
        let auth_provider = source
            .auth
            .as_ref()
            .map(|auth_cfg| {
                r_data_core_workflow::data::adapters::auth::create_auth_provider(auth_cfg)
            })
            .transpose()?;

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
                    return Err(anyhow::anyhow!(
                        "Unsupported source type: {}",
                        source.source_type
                    ));
                }
            };

        let mut stream = source_adapter.fetch(&source_ctx).await?;
        let mut all_data = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
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
                    return Err(anyhow::anyhow!(
                        "Unsupported format type: {}",
                        format.format_type
                    ));
                }
            };

        let payloads = format_handler.parse(&all_data, &format.options)?;
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

impl WorkflowService {
    /// Process staged raw items for a run using the workflow DSL
    ///
    /// # Errors
    /// Returns an error if workflow not found, DSL validation fails, or processing fails
    pub async fn process_staged_items(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> anyhow::Result<(i64, i64)> {
        let wf = self
            .repo
            .get_by_uuid(workflow_uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        // Build DSL program from config; require presence and validation
        let program = match r_data_core_workflow::dsl::DslProgram::from_config(&wf.config) {
            Ok(p) => {
                if let Err(e) = p.validate() {
                    return self
                        .fail_entire_run_due_to_invalid_dsl(run_uuid, e.to_string())
                        .await;
                }
                p
            }
            _ => {
                return self
                    .fail_entire_run_due_to_invalid_dsl(
                        run_uuid,
                        "Missing or invalid DSL configuration".to_string(),
                    )
                    .await;
            }
        };

        let mut processed = 0_i64;
        let mut failed = 0_i64;
        loop {
            let items = self.repo.fetch_staged_raw_items(run_uuid, 200).await?;
            if items.is_empty() {
                break;
            }
            for (item_uuid, payload) in items {
                let success = process_single_item(
                    &program,
                    &payload,
                    item_uuid,
                    run_uuid,
                    wf.versioning_disabled,
                    self.dynamic_entity_service.as_deref(),
                    &self.repo,
                )
                .await?;
                if success {
                    processed += 1;
                } else {
                    failed += 1;
                }
            }
        }
        Ok((processed, failed))
    }

    async fn fail_entire_run_due_to_invalid_dsl(
        &self,
        run_uuid: Uuid,
        message: String,
    ) -> anyhow::Result<(i64, i64)> {
        let _ = self
            .repo
            .insert_run_log(run_uuid, "error", &message, None)
            .await;
        // Mark all queued items as failed to prevent re-processing loops
        loop {
            let items = self.repo.fetch_staged_raw_items(run_uuid, 500).await?;
            if items.is_empty() {
                break;
            }
            for (item_uuid, _payload) in items {
                let _ = self
                    .repo
                    .set_raw_item_status(item_uuid, "failed", Some("Invalid DSL"))
                    .await;
            }
        }
        let _ = self.repo.mark_run_failure(run_uuid, &message).await;
        Err(anyhow::anyhow!(message))
    }
}
