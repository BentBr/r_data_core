use uuid::Uuid;

use crate::workflow::outbox::EnqueueWorkflowFetchUseCase;

use super::WorkflowService;

impl WorkflowService {
    /// List runs for a workflow with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_runs_paginated(
        &self,
        workflow_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> r_data_core_core::error::Result<(
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
    ) -> r_data_core_core::error::Result<(
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
    pub async fn run_exists(&self, run_uuid: Uuid) -> r_data_core_core::error::Result<bool> {
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
    ) -> r_data_core_core::error::Result<(
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
    pub async fn enqueue_run(&self, workflow_uuid: Uuid) -> r_data_core_core::error::Result<Uuid> {
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

    /// Enqueue a workflow run and persist the matching workflow-fetch outbox entry.
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn enqueue_run_with_fetch_outbox(
        &self,
        workflow_uuid: Uuid,
    ) -> r_data_core_core::error::Result<(Uuid, Uuid)> {
        let trigger_id = Uuid::now_v7();
        let (run_uuid, outbox_uuid) = self
            .repo
            .insert_run_queued_with_fetch_outbox(workflow_uuid, trigger_id)
            .await?;
        let _ = self
            .repo
            .insert_run_log(
                run_uuid,
                "info",
                "Run enqueued",
                Some(serde_json::json!({
                    "trigger": trigger_id.to_string(),
                    "outbox_uuid": outbox_uuid.to_string(),
                })),
            )
            .await;
        Ok((run_uuid, outbox_uuid))
    }

    /// Enqueue a workflow run and deliver the matching fetch job.
    ///
    /// When the outbox is enabled, the run is created together with an outbox
    /// record and the worker is responsible for dispatching it.
    /// When the outbox is disabled, the fetch job is written directly to Redis.
    ///
    /// # Errors
    /// Returns an error if the database or queue operation fails.
    pub async fn enqueue_run_for_fetch(
        &self,
        workflow_uuid: Uuid,
        trigger_id: Option<Uuid>,
    ) -> r_data_core_core::error::Result<Uuid> {
        let use_outbox_for_fetch = if let Some(settings_service) = &self.settings_service {
            settings_service.get_outbox_settings().await?.fetch_enabled
        } else {
            self.use_outbox_for_fetch
        };
        let Some(queue) = self.queue.as_deref() else {
            return Err(r_data_core_core::error::Error::Config(
                "WorkflowService queue is not configured".to_string(),
            ));
        };
        EnqueueWorkflowFetchUseCase::new(
            &self.repo,
            queue,
            self.fetch_dispatch_mode(use_outbox_for_fetch),
        )
        .enqueue_run_for_fetch(workflow_uuid, trigger_id)
        .await
    }

    /// Deliver the fetch job for an already created run.
    ///
    /// # Errors
    /// Returns an error if the queue or outbox write fails.
    pub async fn dispatch_fetch_for_existing_run(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<()> {
        let use_outbox_for_fetch = if let Some(settings_service) = &self.settings_service {
            settings_service.get_outbox_settings().await?.fetch_enabled
        } else {
            self.use_outbox_for_fetch
        };
        let Some(queue) = self.queue.as_deref() else {
            return Err(r_data_core_core::error::Error::Config(
                "WorkflowService queue is not configured".to_string(),
            ));
        };
        EnqueueWorkflowFetchUseCase::new(
            &self.repo,
            queue,
            self.fetch_dispatch_mode(use_outbox_for_fetch),
        )
        .dispatch_fetch_for_existing_run(workflow_uuid, run_uuid)
        .await
    }

    /// Mark a run as running
    ///
    /// # Errors
    /// Returns an error if the database update fails
    pub async fn mark_run_running(&self, run_uuid: Uuid) -> r_data_core_core::error::Result<()> {
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
    ) -> r_data_core_core::error::Result<()> {
        self.repo
            .mark_run_success(run_uuid, processed, failed)
            .await
    }

    /// Mark a run as failed with error message
    ///
    /// # Errors
    /// Returns an error if the database update fails
    pub async fn mark_run_failure(
        &self,
        run_uuid: Uuid,
        message: &str,
    ) -> r_data_core_core::error::Result<()> {
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
    ) -> r_data_core_core::error::Result<()> {
        self.repo
            .insert_run_log(run_uuid, level, message, meta)
            .await
    }

    /// Get run status (for async polling)
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_run_status(
        &self,
        run_uuid: Uuid,
    ) -> r_data_core_core::error::Result<Option<String>> {
        self.repo.get_run_status(run_uuid).await
    }
}
