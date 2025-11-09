use crate::services::dynamic_entity_service::DynamicEntityService;
use crate::workflow::data::repository_trait::WorkflowRepositoryTrait;
use crate::workflow::data::Workflow;
use std::sync::Arc;
use uuid::Uuid;

// NOTE: Using API DTOs directly to simplify service signatures as requested.
use crate::api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};
use cron::Schedule;
use std::str::FromStr;

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

    fn validate_dsl_config(cfg: &serde_json::Value) -> anyhow::Result<()> {
        let dsl = cfg
            .get("dsl")
            .ok_or_else(|| anyhow::anyhow!("Invalid workflow configuration: missing 'dsl'"))?;
        let steps = dsl.as_array().ok_or_else(|| {
            anyhow::anyhow!("Invalid workflow configuration: 'dsl' must be an array")
        })?;
        if steps.is_empty() {
            return Err(anyhow::anyhow!(
                "Invalid workflow configuration: 'dsl' must contain at least one step"
            ));
        }
        // Minimal per-step validation
        for (idx, step) in steps.iter().enumerate() {
            let t = step.get("type").and_then(|v| v.as_str()).ok_or_else(|| {
                anyhow::anyhow!(format!("Invalid DSL: step {} missing 'type'", idx))
            })?;
            if t != "map" {
                return Err(anyhow::anyhow!(format!(
                    "Invalid DSL: unsupported step type '{}' at index {}",
                    t, idx
                )));
            }
            if step.get("from").and_then(|v| v.as_str()).is_none() {
                return Err(anyhow::anyhow!(format!(
                    "Invalid DSL: step {} missing 'from'",
                    idx
                )));
            }
            if step.get("to").and_then(|v| v.as_str()).is_none() {
                return Err(anyhow::anyhow!(format!(
                    "Invalid DSL: step {} missing 'to'",
                    idx
                )));
            }
        }
        Ok(())
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
                let format_cfg = Self::csv_format_from_config(&wf.config);
                crate::workflow::data::adapters::import::csv::CsvImportAdapter::parse_inline(
                    &inline,
                    &format_cfg,
                )?
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

        // Determine adapter via unified input.type and input.source.uri
        let input_type = Self::infer_input_type(&wf.config).unwrap_or_else(|| "csv".to_string());
        if let Some(uri) = Self::input_uri_from_config(&wf.config) {
            let body = reqwest::get(&uri).await?.error_for_status()?.text().await?;
            let payloads = match input_type.as_str() {
                "csv" => {
                    let format_cfg = Self::csv_format_from_config(&wf.config);
                    crate::workflow::data::adapters::import::csv::CsvImportAdapter::parse_inline(
                        &body,
                        &format_cfg,
                    )?
                }
                "ndjson" => body
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| serde_json::from_str::<serde_json::Value>(l))
                    .collect::<Result<Vec<_>, _>>()?,
                other => {
                    return Err(anyhow::anyhow!(format!(
                        "Unsupported input type for fetch: {}",
                        other
                    )))
                }
            };
            let staged = self
                .stage_raw_items(workflow_uuid, run_uuid, payloads)
                .await?;
            let _ = self
                .repo
                .insert_run_log(
                    run_uuid,
                    "info",
                    "Fetched and staged",
                    Some(serde_json::json!({ "staged_items": staged, "uri": uri, "input_type": input_type })),
                )
                .await;
            return Ok(staged);
        }

        // Nothing to fetch
        Ok(0)
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
                // Execute steps; on each step, handle ToDef::Entity create/update
                match program.execute(&payload) {
                    Ok(outputs) => {
                        let mut entity_ops_ok = true;
                        for (to_def, produced) in outputs {
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

                                    let result = match mode {
                                        crate::workflow::dsl::EntityWriteMode::Create => {
                                            persist_entity_create(
                                                de_service,
                                                &entity_definition,
                                                &produced,
                                                Some(&path),
                                                run_uuid,
                                            )
                                            .await
                                        }
                                        crate::workflow::dsl::EntityWriteMode::Update => {
                                            persist_entity_update(
                                                de_service,
                                                &entity_definition,
                                                &produced_for_update,
                                                Some(&path),
                                                run_uuid,
                                                update_key.as_ref(),
                                            )
                                            .await
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

async fn persist_entity_create(
    de_service: &DynamicEntityService,
    entity_type: &str,
    produced: &serde_json::Value,
    path: Option<&str>,
    run_uuid: Uuid,
) -> anyhow::Result<()> {
    use crate::entity::dynamic_entity::entity::DynamicEntity;
    use std::sync::Arc;

    // Build field_data as a flat object from produced
    let mut field_data: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    if let Some(obj) = produced.as_object() {
        for (k, v) in obj.iter() {
            field_data.insert(k.clone(), v.clone());
        }
    }
    // Respect path from config if not already set by mapping/normalized
    if let Some(p) = path {
        field_data
            .entry("path".to_string())
            .or_insert_with(|| serde_json::json!(p));
    }

    // Fetch entity definition from the embedded definition service
    let defs = de_service.entity_definition_service();
    let def = defs
        .get_entity_definition_by_entity_type(entity_type)
        .await?;

    // Normalize field names to match definition:
    // exact match preferred; otherwise case-insensitive match
    let reserved: std::collections::HashSet<&'static str> = [
        "uuid",
        "path",
        "parent_uuid",
        "entity_key",
        "created_at",
        "updated_at",
        "created_by",
        "updated_by",
        "published",
        "version",
    ]
    .into_iter()
    .collect();
    let mut def_name_lookup: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for f in &def.fields {
        def_name_lookup.insert(f.name.to_lowercase(), f.name.clone());
    }
    let mut normalized_field_data: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    for (k, v) in field_data.into_iter() {
        if reserved.contains(k.as_str()) {
            // Coerce published from string if needed
            if k == "published" {
                let coerced = match v {
                    serde_json::Value::String(s) => match s.to_lowercase().as_str() {
                        "true" | "1" => serde_json::Value::Bool(true),
                        "false" | "0" => serde_json::Value::Bool(false),
                        _ => serde_json::Value::String(s),
                    },
                    other => other,
                };
                normalized_field_data.insert(k, coerced);
            } else if k == "entity_key" {
                // allow explicit mapping of entity_key
                normalized_field_data.insert(k, v);
            } else if k == "uuid"
                || k == "created_at"
                || k == "updated_at"
                || k == "created_by"
                || k == "updated_by"
            {
                // ignore protected fields from import payload
                continue;
            } else {
                // keep other reserved fields like path, parent_uuid, version if provided
                normalized_field_data.insert(k, v);
            }
            continue;
        }
        if def.fields.iter().any(|f| f.name == k) {
            normalized_field_data.insert(k, v);
        } else if let Some(real) = def_name_lookup.get(&k.to_lowercase()) {
            normalized_field_data.insert(real.clone(), v);
        } else {
            // keep as-is; validator will report unknown field cleanly
            normalized_field_data.insert(k, v);
        }
    }

    // Force uuid generation (repository requires uuid on create)
    normalized_field_data
        .entry("uuid".to_string())
        .or_insert_with(|| serde_json::json!(uuid::Uuid::now_v7().to_string()));

    // Ensure audit fields exist for worker-created entities
    // Use workflow_run UUID for audit fields
    normalized_field_data
        .entry("created_by".to_string())
        .or_insert_with(|| serde_json::json!(run_uuid.to_string()));
    normalized_field_data
        .entry("updated_by".to_string())
        .or_insert_with(|| serde_json::json!(run_uuid.to_string()));

    // Derive entity_key if missing: prefer 'username', then 'email'
    if !normalized_field_data.contains_key("entity_key") {
        if let Some(serde_json::Value::String(u)) = normalized_field_data.get("username") {
            if !u.trim().is_empty() {
                normalized_field_data.insert("entity_key".to_string(), serde_json::json!(u));
            }
        }
        if !normalized_field_data.contains_key("entity_key") {
            if let Some(serde_json::Value::String(e)) = normalized_field_data.get("email") {
                if !e.trim().is_empty() {
                    normalized_field_data.insert("entity_key".to_string(), serde_json::json!(e));
                }
            }
        }
    }
    // If still missing, generate `<entity_type>-<count+1>-<hash8>`
    if !normalized_field_data.contains_key("entity_key") {
        let existing_count = de_service.count_entities(entity_type).await.unwrap_or(0);
        let rand = uuid::Uuid::now_v7().to_string();
        let short = &rand[..8];
        let key = format!("{}-{}-{}", entity_type, existing_count + 1, short);
        normalized_field_data.insert("entity_key".to_string(), serde_json::json!(key));
    }

    let entity = DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data: normalized_field_data,
        definition: Arc::new(def),
    };
    de_service.create_entity(&entity).await?;
    Ok(())
}

async fn persist_entity_update(
    de_service: &DynamicEntityService,
    entity_type: &str,
    produced: &serde_json::Value,
    path: Option<&str>,
    run_uuid: Uuid,
    update_key: Option<&String>,
) -> anyhow::Result<()> {
    use crate::entity::dynamic_entity::entity::DynamicEntity;

    // Build field_data as a flat object from produced (keep original for lookup)
    let mut field_data: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    if let Some(obj) = produced.as_object() {
        for (k, v) in obj.iter() {
            field_data.insert(k.clone(), v.clone());
        }
    }
    let original_field_data = field_data.clone();
    // Respect path from config if not already set by mapping/normalized
    if let Some(p) = path {
        field_data
            .entry("path".to_string())
            .or_insert_with(|| serde_json::json!(p));
    }

    // Fetch entity definition from the embedded definition service
    let defs = de_service.entity_definition_service();
    let def = defs
        .get_entity_definition_by_entity_type(entity_type)
        .await?;

    // Normalize field names to match definition:
    // exact match preferred; otherwise case-insensitive match
    let reserved: std::collections::HashSet<&'static str> = [
        "uuid",
        "path",
        "parent_uuid",
        "entity_key",
        "created_at",
        "updated_at",
        "created_by",
        "updated_by",
        "published",
        "version",
    ]
    .into_iter()
    .collect();
    let mut def_name_lookup: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for f in &def.fields {
        def_name_lookup.insert(f.name.to_lowercase(), f.name.clone());
    }
    let mut normalized_field_data: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    for (k, v) in field_data.into_iter() {
        if reserved.contains(k.as_str()) {
            // Coerce published from string if needed
            if k == "published" {
                let coerced = match v {
                    serde_json::Value::String(s) => match s.to_lowercase().as_str() {
                        "true" | "1" => serde_json::Value::Bool(true),
                        "false" | "0" => serde_json::Value::Bool(false),
                        _ => serde_json::Value::String(s),
                    },
                    other => other,
                };
                normalized_field_data.insert(k, coerced);
            } else if k == "entity_key" {
                // allow explicit mapping of entity_key
                normalized_field_data.insert(k, v);
            } else if k == "created_at" || k == "created_by" {
                // ignore protected fields from import payload (don't allow changing created_at/created_by)
                continue;
            } else {
                // keep other reserved fields like uuid, path, parent_uuid, updated_at, updated_by, version if provided
                normalized_field_data.insert(k, v);
            }
            continue;
        }
        if def.fields.iter().any(|f| f.name == k) {
            normalized_field_data.insert(k, v);
        } else if let Some(real) = def_name_lookup.get(&k.to_lowercase()) {
            normalized_field_data.insert(real.clone(), v);
        } else {
            // keep as-is; validator will report unknown field cleanly
            normalized_field_data.insert(k, v);
        }
    }

    // Find existing entity by uuid, update_key, or entity_key in produced data
    let mut existing_entity: Option<DynamicEntity> = None;

    // First, try to find by UUID if present
    if let Some(serde_json::Value::String(uuid_str)) = normalized_field_data.get("uuid") {
        if let Ok(uuid) = uuid::Uuid::parse_str(uuid_str) {
            if let Ok(Some(entity)) = de_service
                .get_entity_by_uuid(entity_type, &uuid, None)
                .await
            {
                existing_entity = Some(entity);
            }
        }
    }

    // If not found by UUID, try to find by update_key or entity_key
    if existing_entity.is_none() {
        let search_key = if let Some(key_field) = update_key {
            // Use the update_key field name to find the value in produced data
            // First check normalized_field_data, then check original field_data, then check produced directly
            normalized_field_data
                .get(key_field)
                .or_else(|| original_field_data.get(key_field))
                .or_else(|| {
                    if let Some(produced_obj) = produced.as_object() {
                        produced_obj.get(key_field)
                    } else {
                        None
                    }
                })
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else if let Some(serde_json::Value::String(key)) = normalized_field_data.get("entity_key")
        {
            Some(key.clone())
        } else if let Some(serde_json::Value::String(key)) = original_field_data.get("entity_key") {
            Some(key.clone())
        } else if let Some(serde_json::Value::String(key)) = produced.get("entity_key") {
            Some(key.clone())
        } else {
            None
        };

        if let Some(key_value) = search_key {
            // Use filter_entities to find by entity_key
            let mut filters = std::collections::HashMap::new();
            filters.insert("entity_key".to_string(), serde_json::json!(key_value));
            if let Ok(entities) = de_service
                .filter_entities(entity_type, 1, 0, Some(filters), None, None, None)
                .await
            {
                if let Some(entity) = entities.first() {
                    existing_entity = Some(entity.clone());
                }
            }
        }
    }

    // If entity not found, return error
    let mut entity = existing_entity.ok_or_else(|| {
        anyhow::anyhow!("Entity not found for update. Provide uuid or entity_key in the data.")
    })?;

    // Update the entity's field_data with new values
    for (k, v) in normalized_field_data.iter() {
        // Don't overwrite created_at or created_by
        if k != "created_at" && k != "created_by" {
            entity.field_data.insert(k.clone(), v.clone());
        }
    }

    // Set updated_by to run_uuid
    entity.field_data.insert(
        "updated_by".to_string(),
        serde_json::json!(run_uuid.to_string()),
    );

    // Ensure uuid is set (should already be present from existing entity)
    if !entity.field_data.contains_key("uuid") {
        return Err(anyhow::anyhow!("Cannot update entity: missing uuid"));
    }

    de_service.update_entity(&entity).await?;
    Ok(())
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
