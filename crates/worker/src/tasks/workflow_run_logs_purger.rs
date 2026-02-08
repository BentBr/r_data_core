#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use log::{info, warn};

use r_data_core_core::maintenance::task::TaskContext;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_persistence::WorkflowRunRepository;
use r_data_core_services::SettingsService;

/// Maintenance task that purges old workflow runs based on configured settings
pub struct WorkflowRunLogsPurgerTask {
    cron: String,
}

impl WorkflowRunLogsPurgerTask {
    /// Create a new `WorkflowRunLogsPurgerTask` with the given cron expression
    ///
    /// # Arguments
    /// * `cron` - Cron expression for scheduling this task
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // String is not const-constructible
    pub fn new(cron: String) -> Self {
        Self { cron }
    }
}

#[async_trait]
impl MaintenanceTask for WorkflowRunLogsPurgerTask {
    fn name(&self) -> &'static str {
        "workflow_run_logs_purger"
    }

    fn cron(&self) -> &str {
        &self.cron
    }

    async fn execute(
        &self,
        context: &dyn TaskContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("[workflow_run_logs_purger] Starting workflow run logs purging task");

        let pool = context.pool();
        let cache_manager = context.cache_manager_or_default();
        let settings_service = SettingsService::new(pool.clone(), cache_manager);

        let settings = match settings_service.get_workflow_run_log_settings().await {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    "[workflow_run_logs_purger] Failed to retrieve workflow run log settings: {e}"
                );
                return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            }
        };

        let r_data_core_core::settings::WorkflowRunLogSettings {
            enabled,
            max_runs,
            max_age_days,
        } = settings;

        info!(
            "[workflow_run_logs_purger] Settings - enabled: {enabled}, max_runs: {max_runs:?}, max_age_days: {max_age_days:?}"
        );

        if !enabled {
            info!("[workflow_run_logs_purger] Pruning disabled; skipped");
            return Ok(());
        }

        let repo = WorkflowRunRepository::new(pool.clone());

        // Prune by age first if configured
        if let Some(days) = max_age_days {
            info!("[workflow_run_logs_purger] Pruning workflow runs older than {days} days");
            match repo.prune_older_than_days(days).await {
                Ok(count) => {
                    info!(
                        "[workflow_run_logs_purger] Pruned {count} workflow runs older than {days} days"
                    );
                }
                Err(e) => {
                    warn!("[workflow_run_logs_purger] Failed to prune workflow runs by age: {e}");
                    return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
                }
            }
        } else {
            info!(
                "[workflow_run_logs_purger] No max_age_days configured; skipping age-based pruning"
            );
        }

        // Prune by count per workflow if configured
        if let Some(keep) = max_runs {
            info!(
                "[workflow_run_logs_purger] Pruning workflow runs, keeping latest {keep} per workflow"
            );
            match repo.prune_keep_latest_per_workflow(keep).await {
                Ok(count) => {
                    info!(
                        "[workflow_run_logs_purger] Pruned {count} workflow runs by count (kept latest {keep} per workflow)"
                    );
                }
                Err(e) => {
                    warn!("[workflow_run_logs_purger] Failed to prune workflow runs by count: {e}");
                    return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
                }
            }
        } else {
            info!(
                "[workflow_run_logs_purger] No max_runs configured; skipping count-based pruning"
            );
        }

        info!("[workflow_run_logs_purger] Workflow run logs purging task completed successfully");
        Ok(())
    }
}
