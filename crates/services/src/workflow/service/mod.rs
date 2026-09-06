mod crud;
mod execution;
mod runs;
mod staging;

use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::outbox::{FetchDispatchMode, OutboxRetryPolicy};
use crate::{SettingsService, SystemLogService};
use r_data_core_persistence::{OutboxRepositoryTrait, WorkflowRepositoryTrait};
use std::sync::Arc;

pub struct WorkflowService {
    pub(super) repo: Arc<dyn WorkflowRepositoryTrait>,
    pub(super) dynamic_entity_service: Option<Arc<DynamicEntityService>>,
    pub(super) outbox_repository: Option<Arc<dyn OutboxRepositoryTrait>>,
    pub(super) outbox_retry_policy: Option<OutboxRetryPolicy>,
    pub(super) settings_service: Option<Arc<SettingsService>>,
    pub(super) use_outbox_for_fetch: bool,
    pub(super) use_outbox_for_push: bool,
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
            settings_service: None,
            use_outbox_for_fetch: false,
            use_outbox_for_push: false,
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
            settings_service: None,
            use_outbox_for_fetch: false,
            use_outbox_for_push: false,
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
    pub fn with_outbox_repository(
        mut self,
        outbox_repository: Arc<dyn OutboxRepositoryTrait>,
    ) -> Self {
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

    /// Attach the settings service used for runtime outbox mode resolution.
    #[must_use]
    pub fn with_settings_service(mut self, settings_service: Arc<SettingsService>) -> Self {
        self.settings_service = Some(settings_service);
        self
    }

    /// Enable or disable outbox routing for workflow fetch dispatch.
    #[must_use]
    pub const fn with_fetch_outbox(mut self, enabled: bool) -> Self {
        self.use_outbox_for_fetch = enabled;
        self
    }

    /// Enable or disable outbox routing for workflow push dispatch.
    #[must_use]
    pub const fn with_push_outbox(mut self, enabled: bool) -> Self {
        self.use_outbox_for_push = enabled;
        self
    }

    /// Return the configured workflow outbox repository, if enabled.
    #[must_use]
    pub fn outbox_repository(&self) -> Option<&Arc<dyn OutboxRepositoryTrait>> {
        self.outbox_repository.as_ref()
    }

    pub(super) fn fetch_dispatch_mode(&self, fetch_enabled: bool) -> FetchDispatchMode<'_> {
        if fetch_enabled {
            if let Some(repository) = self.outbox_repository.as_deref() {
                return FetchDispatchMode::Outbox {
                    repository,
                    retry_policy: self.outbox_retry_policy.as_ref(),
                };
            }
        }

        FetchDispatchMode::Direct
    }
}

impl WorkflowService {
    /// Execute a DSL program against sample input without changing anything.
    ///
    /// Entity reads and writes go through an in-memory overlay, so mappings,
    /// path templates and lookups run for real and later steps see what
    /// earlier ones produced — but nothing is persisted. Email and outbound
    /// pushes are never attempted, because no overlay can undo them.
    ///
    /// A deployment without a dynamic entity service can still dry-run
    /// programs that never touch entities; steps that would need one are
    /// reported as unavailable rather than silently skipped.
    ///
    /// # Errors
    /// Returns an error if the program is invalid or a step fails.
    pub async fn dry_run(
        &self,
        program: &r_data_core_workflow::dsl::DslProgram,
        input: &serde_json::Value,
    ) -> r_data_core_core::error::Result<r_data_core_core::dto::dsl::DryRunResponse> {
        crate::workflow::dry_run::execute_dry_run(
            program,
            input,
            crate::workflow::dry_run::DryRunDeps {
                entity_service: self.dynamic_entity_service.as_deref(),
                workflow_repository: &self.repo,
                jwt_secret: self.jwt_secret.as_deref(),
                jwt_expiration: self.jwt_expiration,
            },
        )
        .await
    }
}
