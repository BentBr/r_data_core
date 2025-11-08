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

    fn infer_input_type(cfg: &serde_json::Value) -> Option<String> {
        // Required structure: { "input": { "type": "csv" | "ndjson", "format": {...}, "source": {...} } }
        cfg.pointer("/input/type").and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    fn csv_format_from_config(cfg: &serde_json::Value) -> serde_json::Value {
        cfg.pointer("/input/format")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({ "has_header": true, "delimiter": ",", "quote": "\"" }))
    }

    fn input_uri_from_config(cfg: &serde_json::Value) -> Option<String> {
        cfg.pointer("/input/source/uri").and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    pub async fn list(&self) -> anyhow::Result<Vec<Workflow>> {
        self.repo.list_all().await
    }

    pub async fn get(&self, uuid: Uuid) -> anyhow::Result<Option<Workflow>> {
        self.repo.get_by_uuid(uuid).await
    }

    pub async fn create(&self, req: &CreateWorkflowRequest) -> anyhow::Result<Uuid> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| anyhow::anyhow!("Invalid cron schedule: {}", e))?;
        }
        self.repo.create(req).await
    }

    pub async fn update(&self, uuid: Uuid, req: &UpdateWorkflowRequest) -> anyhow::Result<()> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| anyhow::anyhow!("Invalid cron schedule: {}", e))?;
        }
        self.repo.update(uuid, req).await
    }

    pub async fn delete(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.repo.delete(uuid).await
    }

    pub async fn list_paginated(&self, limit: i64, offset: i64) -> anyhow::Result<(Vec<Workflow>, i64)> {
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
    ) -> anyhow::Result<(Vec<(Uuid, String, Option<String>, Option<String>, Option<i64>, Option<i64>)>, i64)> {
        self.repo
            .list_runs_paginated(workflow_uuid, limit, offset)
            .await
    }

    pub async fn list_run_logs_paginated(
        &self,
        run_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<(Uuid, String, String, String, Option<serde_json::Value>)>, i64)> {
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
    ) -> anyhow::Result<(Vec<(Uuid, String, Option<String>, Option<String>, Option<i64>, Option<i64>)>, i64)> {
        self.repo.list_all_runs_paginated(limit, offset).await
    }

    pub async fn enqueue_run(&self, workflow_uuid: Uuid) -> anyhow::Result<Uuid> {
        let trigger_id = Uuid::now_v7();
        let run_uuid = self.repo.insert_run_queued(workflow_uuid, trigger_id).await?;
        // Optional: write an initial log entry
        let _ = self
            .repo
            .insert_run_log(run_uuid, "info", "Run enqueued", Some(serde_json::json!({ "trigger": trigger_id.to_string() })))
            .await;
        Ok(run_uuid)
    }

    pub async fn stage_raw_items(&self, workflow_uuid: Uuid, run_uuid: Uuid, payloads: Vec<serde_json::Value>) -> anyhow::Result<i64> {
        self.repo.insert_raw_items(workflow_uuid, run_uuid, payloads).await
    }

    /// Handle a CSV upload for a run-now execution:
    /// - creates a run (queued)
    /// - parses CSV (expects headers)
    /// - stages rows as raw items
    /// - writes a staging log
    pub async fn run_now_upload_csv(&self, workflow_uuid: Uuid, bytes: &[u8]) -> anyhow::Result<(Uuid, i64)> {
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
                crate::workflow::data::adapters::import::csv::CsvImportAdapter::parse_inline(&inline, &format_cfg)?
            }
            "ndjson" => {
                inline
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| serde_json::from_str::<serde_json::Value>(l))
                    .collect::<Result<Vec<_>, _>>()?
            }
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
            .insert_run_log(run_uuid, "info", "Upload staged", Some(serde_json::json!({ "staged_items": staged, "input_type": input_type })))
            .await;
        Ok((run_uuid, staged))
    }

    /// Fetch from configured source (URI) and stage items using the appropriate adapter (csv or ndjson)
    pub async fn fetch_and_stage_from_config(&self, workflow_uuid: Uuid, run_uuid: Uuid) -> anyhow::Result<i64> {
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
                    crate::workflow::data::adapters::import::csv::CsvImportAdapter::parse_inline(&body, &format_cfg)?
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
            let staged = self.stage_raw_items(workflow_uuid, run_uuid, payloads).await?;
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

    /// Process staged raw items for a run using the workflow DSL (stub: pass-through mapping)
    pub async fn process_staged_items(&self, workflow_uuid: Uuid, run_uuid: Uuid) -> anyhow::Result<(i64, i64)> {
        let wf = self
            .repo
            .get_by_uuid(workflow_uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?;

        // For now, accept no-op if dsl absent; later require config.dsl
        let _dsl = wf.config.get("dsl").cloned().unwrap_or(serde_json::json!([]));

        let mut processed = 0_i64;
        let mut failed = 0_i64;
        loop {
            let items = self.repo.fetch_staged_raw_items(run_uuid, 200).await?;
            if items.is_empty() {
                break;
            }
            for (item_uuid, payload) in items {
                // TODO: apply DSL steps; for now, pass-through
                let _transformed = payload;
                // TODO: upsert/update entities based on DSL target
                // Mark processed
                if let Err(e) = self.repo.set_raw_item_status(item_uuid, "processed", None).await {
                    let _ = self
                        .repo
                        .insert_run_log(
                            run_uuid,
                            "error",
                            "Failed to mark item processed",
                            Some(serde_json::json!({ "item_uuid": item_uuid, "error": e.to_string() })),
                        )
                        .await;
                    failed += 1;
                } else {
                    processed += 1;
                }
            }
        }
        Ok((processed, failed))
    }
}
