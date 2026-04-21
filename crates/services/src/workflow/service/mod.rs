mod execution;
mod staging;

use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::outbox::{dispatch_workflow_fetch_job, OutboxRetryPolicy};
use crate::SystemLogService;
use cron::Schedule;
use r_data_core_core::system_log::SystemLogResourceType;
use r_data_core_persistence::OutboxRepository;
use r_data_core_persistence::WorkflowRepositoryTrait;
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::FetchAndStageJob;
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core_workflow::data::Workflow;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub struct WorkflowService {
    pub(super) repo: Arc<dyn WorkflowRepositoryTrait>,
    pub(super) dynamic_entity_service: Option<Arc<DynamicEntityService>>,
    pub(super) outbox_repository: Option<Arc<OutboxRepository>>,
    pub(super) outbox_retry_policy: Option<OutboxRetryPolicy>,
    /// JWT signing secret (base, before entity suffix) for authenticate transforms
    pub jwt_secret: Option<String>,
    /// Default JWT expiration in seconds (from `JWT_EXPIRATION` env)
    pub jwt_expiration: u64,
    /// Mail service for sending emails from workflow transforms and email outputs
    pub mail_service: Option<Arc<crate::mail::MailService>>,
    /// Job queue for enqueuing email (and other) jobs
    pub queue: Option<Arc<dyn r_data_core_workflow::data::job_queue::JobQueue>>,
    /// System log service for audit logging
    pub system_log: Option<Arc<SystemLogService>>,
}

/// Default JWT expiration: 24 hours
const DEFAULT_JWT_EXPIRATION: u64 = 86_400;

impl WorkflowService {
    pub fn new(repo: Arc<dyn WorkflowRepositoryTrait>) -> Self {
        Self {
            repo,
            dynamic_entity_service: None,
            outbox_repository: None,
            outbox_retry_policy: None,
            jwt_secret: None,
            jwt_expiration: DEFAULT_JWT_EXPIRATION,
            mail_service: None,
            queue: None,
            system_log: None,
        }
    }

    pub fn new_with_entities(
        repo: Arc<dyn WorkflowRepositoryTrait>,
        dynamic_entity_service: Arc<DynamicEntityService>,
    ) -> Self {
        Self {
            repo,
            dynamic_entity_service: Some(dynamic_entity_service),
            outbox_repository: None,
            outbox_retry_policy: None,
            jwt_secret: None,
            jwt_expiration: DEFAULT_JWT_EXPIRATION,
            mail_service: None,
            queue: None,
            system_log: None,
        }
    }

    /// Set JWT configuration for authenticate transforms
    #[must_use]
    pub fn with_jwt_config(mut self, secret: Option<String>, expiration: u64) -> Self {
        self.jwt_secret = secret;
        self.jwt_expiration = expiration;
        self
    }

    /// Set the mail service for email transforms and email to-targets
    #[must_use]
    pub fn with_mail_service(mut self, svc: Option<Arc<crate::mail::MailService>>) -> Self {
        self.mail_service = svc;
        self
    }

    /// Set the job queue for enqueueing email (and other) jobs
    #[must_use]
    pub fn with_queue(
        mut self,
        queue: Option<Arc<dyn r_data_core_workflow::data::job_queue::JobQueue>>,
    ) -> Self {
        self.queue = queue;
        self
    }

    /// Set the system log service for audit logging
    #[must_use]
    pub fn with_system_log(mut self, log: Arc<SystemLogService>) -> Self {
        self.system_log = Some(log);
        self
    }

    /// Attach an outbox repository for deferred workflow deliveries.
    #[must_use]
    pub fn with_outbox_repository(mut self, outbox_repository: Arc<OutboxRepository>) -> Self {
        self.outbox_repository = Some(outbox_repository);
        self
    }

    /// Attach a retry policy for deferred workflow deliveries.
    #[must_use]
    pub const fn with_outbox_retry_policy(
        mut self,
        outbox_retry_policy: OutboxRetryPolicy,
    ) -> Self {
        self.outbox_retry_policy = Some(outbox_retry_policy);
        self
    }

    /// Return the configured workflow outbox repository, if enabled.
    #[must_use]
    pub const fn outbox_repository(&self) -> Option<&Arc<OutboxRepository>> {
        self.outbox_repository.as_ref()
    }

    /// List all workflows
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list(&self) -> r_data_core_core::error::Result<Vec<Workflow>> {
        self.repo.list_all().await
    }

    /// Get a workflow by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get(&self, uuid: Uuid) -> r_data_core_core::error::Result<Option<Workflow>> {
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
    ) -> r_data_core_core::error::Result<Uuid> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!("Invalid cron schedule: {e}"))
            })?;
        }
        // Strict DSL: parse and validate
        let program =
            r_data_core_workflow::dsl::DslProgram::from_config(&req.config).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!(
                    "Invalid workflow DSL configuration: {e}"
                ))
            })?;
        program.validate().map_err(|e| {
            r_data_core_core::error::Error::Validation(format!(
                "Workflow DSL validation failed: {e}"
            ))
        })?;
        let uuid = self.repo.create(req, created_by).await?;

        if let Some(ref log) = self.system_log {
            log.log_entity_created(
                Some(created_by),
                SystemLogResourceType::Workflow,
                uuid,
                &format!("Workflow '{}' created", req.name),
                Some(serde_json::json!({"name": req.name})),
            )
            .await;
        }

        Ok(uuid)
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
    ) -> r_data_core_core::error::Result<()> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!("Invalid cron schedule: {e}"))
            })?;
        }
        // Strict DSL: parse and validate
        let program =
            r_data_core_workflow::dsl::DslProgram::from_config(&req.config).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!(
                    "Invalid workflow DSL configuration: {e}"
                ))
            })?;
        program.validate().map_err(|e| {
            r_data_core_core::error::Error::Validation(format!(
                "Workflow DSL validation failed: {e}"
            ))
        })?;
        self.repo.update(uuid, req, updated_by).await?;

        if let Some(ref log) = self.system_log {
            log.log_entity_updated(
                Some(updated_by),
                SystemLogResourceType::Workflow,
                uuid,
                &format!("Workflow '{}' updated", req.name),
                Some(serde_json::json!({"name": req.name})),
            )
            .await;
        }

        Ok(())
    }

    /// Delete a workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn delete(
        &self,
        uuid: Uuid,
        actor_uuid: Uuid,
    ) -> r_data_core_core::error::Result<()> {
        // Capture the workflow name before deletion for audit log
        let workflow_name = self
            .repo
            .get_by_uuid(uuid)
            .await
            .ok()
            .flatten()
            .map_or_else(|| uuid.to_string(), |w| w.name);

        self.repo.delete(uuid).await?;

        if let Some(ref log) = self.system_log {
            log.log_entity_deleted(
                Some(actor_uuid),
                SystemLogResourceType::Workflow,
                uuid,
                &format!("Workflow '{workflow_name}' deleted"),
                Some(serde_json::json!({"name": workflow_name})),
            )
            .await;
        }

        Ok(())
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
    ) -> r_data_core_core::error::Result<(Vec<Workflow>, i64)> {
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
    ) -> r_data_core_core::error::Result<(
        (Vec<Workflow>, i64),
        crate::query_validation::ValidatedListQuery,
    )> {
        use crate::query_validation::validate_list_query;

        let validated =
            validate_list_query(params, "workflows", field_validator, 20, 100, true, &[])
                .await
                .map_err(|e| {
                    r_data_core_core::error::Error::Validation(format!(
                        "Query validation failed: {e}"
                    ))
                })?;

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
        queue: &dyn JobQueue,
        trigger_id: Option<Uuid>,
    ) -> r_data_core_core::error::Result<Uuid> {
        let trigger_id = trigger_id.unwrap_or_else(Uuid::now_v7);
        if let Some(outbox_repo) = self.outbox_repository.as_deref() {
            let (run_uuid, outbox_uuid) = self
                .repo
                .insert_run_queued_with_fetch_outbox(workflow_uuid, trigger_id)
                .await?;
            dispatch_workflow_fetch_job(
                queue,
                outbox_repo,
                workflow_uuid,
                run_uuid,
                outbox_uuid,
                1,
                None,
                self.outbox_retry_policy.as_ref(),
            )
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
            return Ok(run_uuid);
        }

        let run_uuid = self
            .repo
            .insert_run_queued(workflow_uuid, trigger_id)
            .await?;
        queue
            .enqueue_fetch(FetchAndStageJob {
                workflow_id: workflow_uuid,
                trigger_id: Some(run_uuid),
            })
            .await?;
        let _ = self
            .repo
            .insert_run_log(
                run_uuid,
                "info",
                "Run enqueued",
                Some(serde_json::json!({
                    "trigger": trigger_id.to_string(),
                })),
            )
            .await;
        Ok(run_uuid)
    }

    /// Deliver the fetch job for an already created run.
    ///
    /// # Errors
    /// Returns an error if the queue or outbox write fails.
    pub async fn dispatch_fetch_for_existing_run(
        &self,
        workflow_uuid: Uuid,
        run_uuid: Uuid,
        queue: &dyn JobQueue,
    ) -> r_data_core_core::error::Result<()> {
        if let Some(outbox_repo) = self.outbox_repository.as_deref() {
            let outbox_uuid = outbox_repo
                .insert_workflow_fetch_enqueue(workflow_uuid, run_uuid)
                .await?;
            dispatch_workflow_fetch_job(
                queue,
                outbox_repo,
                workflow_uuid,
                run_uuid,
                outbox_uuid,
                1,
                None,
                self.outbox_retry_policy.as_ref(),
            )
            .await?;
            let _ = self
                .repo
                .insert_run_log(
                    run_uuid,
                    "info",
                    "Run enqueued",
                    Some(serde_json::json!({
                        "run_uuid": run_uuid.to_string(),
                        "outbox_uuid": outbox_uuid.to_string(),
                    })),
                )
                .await;
            return Ok(());
        }

        queue
            .enqueue_fetch(FetchAndStageJob {
                workflow_id: workflow_uuid,
                trigger_id: Some(run_uuid),
            })
            .await?;
        let _ = self
            .repo
            .insert_run_log(
                run_uuid,
                "info",
                "Run enqueued",
                Some(serde_json::json!({
                    "run_uuid": run_uuid.to_string(),
                })),
            )
            .await;
        Ok(())
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
