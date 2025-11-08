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
}

impl WorkflowService {
    pub fn new(repo: Arc<dyn WorkflowRepositoryTrait>) -> Self {
        Self { repo }
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

    pub async fn create(&self, req: &CreateWorkflowRequest) -> anyhow::Result<Uuid> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr)
                .map_err(|e| anyhow::anyhow!("Invalid cron schedule: {}", e))?;
        }
        // Strict DSL: parse and validate
        let program = crate::workflow::dsl::DslProgram::from_config(&req.config)?;
        program.validate()?;
        self.repo.create(req).await
    }

    pub async fn update(&self, uuid: Uuid, req: &UpdateWorkflowRequest) -> anyhow::Result<()> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr)
                .map_err(|e| anyhow::anyhow!("Invalid cron schedule: {}", e))?;
        }
        // Strict DSL: parse and validate
        let program = crate::workflow::dsl::DslProgram::from_config(&req.config)?;
        program.validate()?;
        self.repo.update(uuid, req).await
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
                // Apply DSL steps; if it fails, mark item as error and continue
                match program.apply(&payload) {
                    Ok(transformed) => {
                        // TODO: upsert/update entities based on `transformed`
                        let _ = self
                            .repo
                            .insert_run_log(
                                run_uuid,
                                "debug",
                                "Transformed item",
                                Some(serde_json::json!({ "item_uuid": item_uuid, "transformed": transformed })),
                            )
                            .await;
                        // Mark processed
                        if let Err(e) = self
                            .repo
                            .set_raw_item_status(item_uuid, "processed", None)
                            .await
                        {
                            // Try to unwrap sqlx database details for better diagnostics
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
                                "DSL apply failed for item; item marked as error",
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
