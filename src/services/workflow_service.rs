use r_data_core_api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core_services::DynamicEntityService;
use crate::services::workflow::entity_persistence::{
    create_entity, create_or_update_entity, update_entity, PersistenceContext,
};
use crate::workflow::data::repository_trait::WorkflowRepositoryTrait;
use crate::workflow::data::Workflow;
use cron::Schedule;
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
            .map(|s| s.to_string())
    }

    fn csv_format_from_config(cfg: &serde_json::Value) -> serde_json::Value {
        cfg.pointer("/input/format").cloned().unwrap_or_else(
            || serde_json::json!({ "has_header": true, "delimiter": ",", "quote": "\"" }),
        )
    }

    #[allow(dead_code)]
    fn input_uri_from_config(cfg: &serde_json::Value) -> Option<String> {
        cfg.pointer("/input/source/uri")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub async fn list(&self) -> anyhow::Result<Vec<Workflow>> {
        self.repo.list_all().await
    }

    pub async fn get(&self, uuid: Uuid) -> anyhow::Result<Option<Workflow>> {
        self.repo.get_by_uuid(uuid).await
    }

    pub async fn create(
        &self,
        req: &CreateWorkflowRequest,
        created_by: Uuid,
    ) -> anyhow::Result<Uuid> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr)
                .map_err(|e| anyhow::anyhow!("Invalid cron schedule: {}", e))?;
        }
        // Strict DSL: parse and validate
        let program = crate::workflow::dsl::DslProgram::from_config(&req.config)?;
        program.validate()?;
        self.repo.create(req, created_by).await
    }

    pub async fn update(
        &self,
        uuid: Uuid,
        req: &UpdateWorkflowRequest,
        updated_by: Uuid,
    ) -> anyhow::Result<()> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr)
                .map_err(|e| anyhow::anyhow!("Invalid cron schedule: {}", e))?;
        }
        // Strict DSL: parse and validate
        let program = crate::workflow::dsl::DslProgram::from_config(&req.config)?;
        program.validate()?;
        self.repo.update(uuid, req, updated_by).await
    }

    pub async fn delete(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.repo.delete(uuid).await
    }

    pub async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<Workflow>, i64)> {
        let (items, total) = tokio::try_join!(
            self.repo.list_paginated(limit, offset),
            self.repo.count_all()
        )?;
        Ok((items, total))
    }

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

    pub async fn run_exists(&self, run_uuid: Uuid) -> anyhow::Result<bool> {
        self.repo.run_exists(run_uuid).await
    }

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
                use crate::workflow::data::adapters::format::FormatHandler;
                let format_cfg = Self::csv_format_from_config(&wf.config);
                crate::workflow::data::adapters::format::csv::CsvFormatHandler::new()
                    .parse(inline.as_bytes(), &format_cfg)?
            }
            "ndjson" => inline
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| serde_json::from_str::<serde_json::Value>(l))
                .collect::<Result<Vec<_>, _>>()?,
            other => {
                return Err(anyhow::anyhow!(format!(
                    "Unsupported input type for upload: {}",
                    other
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
        let program = crate::workflow::dsl::DslProgram::from_config(&wf.config)?;
        let format_type = program
            .steps
            .first()
            .and_then(|step| {
                if let crate::workflow::dsl::FromDef::Format { format, .. } = &step.from {
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
                        if let crate::workflow::dsl::FromDef::Format { format, .. } = &step.from {
                            Some(format.options.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| serde_json::json!({}));
                {
                    use crate::workflow::data::adapters::format::FormatHandler;
                    crate::workflow::data::adapters::format::csv::CsvFormatHandler::new()
                        .parse(bytes, &format_cfg)?
                }
            }
            "json" => {
                let format_cfg = program
                    .steps
                    .first()
                    .and_then(|step| {
                        if let crate::workflow::dsl::FromDef::Format { format, .. } = &step.from {
                            Some(format.options.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| serde_json::json!({}));
                {
                    use crate::workflow::data::adapters::format::FormatHandler;
                    crate::workflow::data::adapters::format::json::JsonFormatHandler::new()
                        .parse(bytes, &format_cfg)?
                }
            }
            other => {
                return Err(anyhow::anyhow!(format!(
                    "Unsupported input type for upload: {}",
                    other
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
        let program = crate::workflow::dsl::DslProgram::from_config(&wf.config)
            .map_err(|e| anyhow::anyhow!("Failed to parse DSL for fetch: {}", e))?;

        // Find Format-based FromDef steps that need fetching
        let mut total_staged = 0_i64;
        for step in &program.steps {
            if let crate::workflow::dsl::FromDef::Format { source, format, .. } = &step.from {
                // Skip "api" source type (handled by POST endpoint)
                if source.source_type == "api" {
                    continue;
                }

                // Create auth provider
                let auth_provider = source
                    .auth
                    .as_ref()
                    .map(|auth_cfg| {
                        crate::workflow::data::adapters::auth::create_auth_provider(auth_cfg)
                    })
                    .transpose()?;

                // Create source context
                let source_ctx = crate::workflow::data::adapters::source::SourceContext {
                    auth: auth_provider,
                    config: source.config.clone(),
                };

                // Get appropriate source based on source_type
                let source_adapter: Box<dyn crate::workflow::data::adapters::source::DataSource> =
                    match source.source_type.as_str() {
                        "uri" => {
                            Box::new(crate::workflow::data::adapters::source::uri::UriSource::new())
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Unsupported source type: {}",
                                source.source_type
                            ));
                        }
                    };

                // Fetch data
                let mut stream = source_adapter.fetch(&source_ctx).await?;
                use futures::StreamExt;
                let mut all_data = Vec::new();
                while let Some(chunk_result) = stream.next().await {
                    let chunk = chunk_result?;
                    all_data.extend_from_slice(&chunk);
                }

                // Get format handler
                let format_handler: Box<
                    dyn crate::workflow::data::adapters::format::FormatHandler,
                > = match format.format_type.as_str() {
                    "csv" => Box::new(
                        crate::workflow::data::adapters::format::csv::CsvFormatHandler::new(),
                    ),
                    "json" => Box::new(
                        crate::workflow::data::adapters::format::json::JsonFormatHandler::new(),
                    ),
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Unsupported format type: {}",
                            format.format_type
                        ));
                    }
                };

                // Parse data
                let payloads = format_handler.parse(&all_data, &format.options)?;

                // Stage items
                let staged = self
                    .stage_raw_items(workflow_uuid, run_uuid, payloads)
                    .await?;
                total_staged += staged;

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
            }
        }

        Ok(total_staged)
    }

    /// Process staged raw items for a run using the workflow DSL
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
        let program = match crate::workflow::dsl::DslProgram::from_config(&wf.config) {
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
                // Execute steps; on each step, handle ToDef::Entity create/update and ToDef::Format with Push
                match program.execute(&payload) {
                    Ok(outputs) => {
                        let mut entity_ops_ok = true;
                        for (to_def, produced) in outputs {
                            // Handle Format outputs with Push mode
                            if let crate::workflow::dsl::ToDef::Format { output, format, .. } =
                                &to_def
                            {
                                if let crate::workflow::dsl::OutputMode::Push {
                                    destination,
                                    method,
                                } = output
                                {
                                    // Serialize data using format handler
                                    let format_handler: Box<
                                        dyn crate::workflow::data::adapters::format::FormatHandler,
                                    > = match format.format_type.as_str() {
                                        "csv" => Box::new(
                                            crate::workflow::data::adapters::format::csv::CsvFormatHandler::new(),
                                        ),
                                        "json" => Box::new(
                                            crate::workflow::data::adapters::format::json::JsonFormatHandler::new(),
                                        ),
                                        _ => {
                                            let _ = self
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
                                                .await;
                                            entity_ops_ok = false;
                                            break;
                                        }
                                    };

                                    // Serialize to bytes (clone produced since it may be used later for Entity outputs)
                                    let data_bytes = match format_handler
                                        .serialize(&[produced.clone()], &format.options)
                                    {
                                        Ok(bytes) => bytes,
                                        Err(e) => {
                                            let _ = self
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
                                            entity_ops_ok = false;
                                            break;
                                        }
                                    };

                                    // Create auth provider
                                    let auth_provider = destination
                                        .auth
                                        .as_ref()
                                        .map(|auth_cfg| {
                                            crate::workflow::data::adapters::auth::create_auth_provider(auth_cfg)
                                        })
                                        .transpose()?;

                                    // Create destination context
                                    let dest_ctx = crate::workflow::data::adapters::destination::DestinationContext {
                                        auth: auth_provider,
                                        method: *method,
                                        config: destination.config.clone(),
                                    };

                                    // Get appropriate destination based on destination_type
                                    let dest_adapter: Box<
                                        dyn crate::workflow::data::adapters::destination::DataDestination,
                                    > = match destination.destination_type.as_str() {
                                        "uri" => Box::new(
                                            crate::workflow::data::adapters::destination::uri::UriDestination::new(),
                                        ),
                                        _ => {
                                            let _ = self
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
                                            entity_ops_ok = false;
                                            break;
                                        }
                                    };

                                    // Push data
                                    if let Err(e) = dest_adapter.push(&dest_ctx, data_bytes).await {
                                        let _ = self
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
                                        entity_ops_ok = false;
                                        break;
                                    }
                                }
                            }

                            // Handle Entity outputs
                            if let crate::workflow::dsl::ToDef::Entity {
                                entity_definition,
                                path,
                                mode,
                                identify: _,
                                update_key,
                                mapping: _,
                            } = to_def
                            {
                                if let Some(de_service) = &self.dynamic_entity_service {
                                    // For update mode, merge payload into produced to ensure update_key is available
                                    let produced_for_update = if matches!(
                                        mode,
                                        crate::workflow::dsl::EntityWriteMode::Update
                                            | crate::workflow::dsl::EntityWriteMode::CreateOrUpdate
                                    ) {
                                        let mut merged = produced.clone();
                                        if let (Some(merged_obj), Some(payload_obj)) =
                                            (merged.as_object_mut(), payload.as_object())
                                        {
                                            // Merge payload fields into produced (payload takes precedence for update_key)
                                            for (k, v) in payload_obj {
                                                if k == "entity_key"
                                                    || update_key
                                                        .as_ref()
                                                        .map(|uk| k == uk)
                                                        .unwrap_or(false)
                                                {
                                                    merged_obj.insert(k.clone(), v.clone());
                                                }
                                            }
                                        }
                                        merged
                                    } else {
                                        produced.clone()
                                    };

                                    let ctx = PersistenceContext {
                                        entity_type: entity_definition.clone(),
                                        produced: produced_for_update.clone(),
                                        path: Some(path.clone()),
                                        run_uuid,
                                        update_key: update_key.clone(),
                                        skip_versioning: wf.versioning_disabled,
                                    };

                                    let result = match mode {
                                        crate::workflow::dsl::EntityWriteMode::Create => {
                                            let create_ctx = PersistenceContext {
                                                entity_type: entity_definition.clone(),
                                                produced: produced.clone(),
                                                path: Some(path.clone()),
                                                run_uuid,
                                                update_key: None,
                                                skip_versioning: wf.versioning_disabled,
                                            };
                                            create_entity(de_service, &create_ctx).await
                                        }
                                        crate::workflow::dsl::EntityWriteMode::Update => {
                                            update_entity(de_service, &ctx).await
                                        }
                                        crate::workflow::dsl::EntityWriteMode::CreateOrUpdate => {
                                            create_or_update_entity(de_service, &ctx).await
                                        }
                                    };
                                    if let Err(e) = result {
                                        let operation = match mode {
                                            crate::workflow::dsl::EntityWriteMode::Create => {
                                                "create"
                                            }
                                            crate::workflow::dsl::EntityWriteMode::Update => {
                                                "update"
                                            }
                                            crate::workflow::dsl::EntityWriteMode::CreateOrUpdate => {
                                                "create_or_update"
                                            }
                                        };
                                        let _ = self
                                            .repo
                                            .insert_run_log(
                                                run_uuid,
                                                "error",
                                                &format!("Entity {} failed", operation),
                                                Some(serde_json::json!({
                                                    "item_uuid": item_uuid,
                                                    "entity_type": entity_definition,
                                                    "mode": format!("{:?}", mode),
                                                    "error": e.to_string()
                                                })),
                                            )
                                            .await;
                                        entity_ops_ok = false;
                                        break;
                                    }
                                }
                            }
                        }

                        if entity_ops_ok {
                            // Mark processed
                            if let Err(e) = self
                                .repo
                                .set_raw_item_status(item_uuid, "processed", None)
                                .await
                            {
                                let db_meta = extract_sqlx_meta(&e);
                                let _ = self
                                    .repo
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
                                failed += 1;
                            } else {
                                processed += 1;
                            }
                        } else {
                            // Entity op failed, mark item failed
                            let _ = self
                                .repo
                                .set_raw_item_status(
                                    item_uuid,
                                    "failed",
                                    Some("entity operation failed"),
                                )
                                .await;
                            failed += 1;
                        }
                    }
                    Err(e) => {
                        // Mark item as error to prevent reprocessing
                        if let Err(set_err) = self
                            .repo
                            .set_raw_item_status(item_uuid, "failed", Some(&e.to_string()))
                            .await
                        {
                            let db_meta = extract_sqlx_meta(&set_err);
                            let _ = self
                                .repo
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
                        let _ = self
                            .repo
                            .insert_run_log(
                                run_uuid,
                                "error",
                                "DSL execute failed for item; item marked as error",
                                Some(serde_json::json!({ "item_uuid": item_uuid, "error": e.to_string() })),
                            )
                            .await;
                        failed += 1;
                    }
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

fn extract_sqlx_meta(e: &anyhow::Error) -> serde_json::Value {
    // Walk the error chain and extract sqlx::Error::Database details if present
    // Fall back to debug formatting of the full chain
    let mut code: Option<String> = None;
    let mut message: Option<String> = None;

    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(e.as_ref());
    while let Some(err) = cause {
        if let Some(sqlx_err) = err.downcast_ref::<sqlx::Error>() {
            if let sqlx::Error::Database(db_err) = sqlx_err {
                code = db_err.code().map(|s| s.to_string());
                message = Some(db_err.message().to_string());
                break;
            }
        }
        cause = err.source();
    }

    serde_json::json!({
        "code": code,
        "message": message,
        "chain": format!("{:?}", e),
    })
}
