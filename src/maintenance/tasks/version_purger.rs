use async_trait::async_trait;
use log::{info, warn};

use crate::api::admin::entity_definitions::versioning_repository::EntityDefinitionVersioningRepository;
use crate::entity::version_repository::VersionRepository;
use crate::error::Result;
use crate::maintenance::task::{MaintenanceTask, TaskContext};
use crate::services::settings_service::SettingsService;
use crate::system_settings::entity_versioning::EntityVersioningSettings;
use crate::versioning::purger_trait::VersionPurger;
use crate::workflow::data::versioning_repository::WorkflowVersioningRepository;

/// Maintenance task that purges old entity versions based on configured settings
pub struct VersionPurgerTask {
    cron: String,
}

impl VersionPurgerTask {
    /// Create a new VersionPurgerTask from MaintenanceConfig
    pub fn new(version_purger_cron: String) -> Self {
        Self {
            cron: version_purger_cron,
        }
    }
}

#[async_trait]
impl MaintenanceTask for VersionPurgerTask {
    fn name(&self) -> &'static str {
        "version_purger"
    }

    fn cron(&self) -> &str {
        &self.cron
    }

    async fn execute(&self, context: &TaskContext) -> Result<()> {
        info!("[version_purger] Starting version purging task");

        let settings_service = SettingsService::new(context.pool.clone(), context.cache.clone());
        let EntityVersioningSettings {
            enabled,
            max_versions,
            max_age_days,
        } = settings_service
            .get_entity_versioning_settings()
            .await
            .map_err(|e| {
                warn!(
                    "[version_purger] Failed to retrieve entity versioning settings: {}",
                    e
                );
                e
            })?;

        info!(
            "[version_purger] Entity versioning settings - enabled: {}, max_versions: {:?}, max_age_days: {:?}",
            enabled, max_versions, max_age_days
        );

        if !enabled {
            info!("[version_purger] Entity versioning disabled; prune skipped");
            return Ok(());
        }

        // Initialize all three version repositories
        let entity_repo = VersionRepository::new(context.pool.clone());
        let workflow_repo = WorkflowVersioningRepository::new(context.pool.clone());
        let definition_repo = EntityDefinitionVersioningRepository::new(context.pool.clone());

        let repositories: Vec<Box<dyn VersionPurger>> = vec![
            Box::new(entity_repo),
            Box::new(workflow_repo),
            Box::new(definition_repo),
        ];

        // Prune by age first if configured
        if let Some(days) = max_age_days {
            info!(
                "[version_purger] Pruning versions older than {} days across all repositories",
                days
            );
            for repo in &repositories {
                let repo_name = repo.repository_name();
                match repo.prune_older_than_days(days).await {
                    Ok(count) => {
                        info!(
                            "[version_purger] Pruned {} versions older than {} days from {}",
                            count, days, repo_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "[version_purger] Failed to prune versions by age from {}: {}",
                            repo_name, e
                        );
                        return Err(e);
                    }
                }
            }
        } else {
            info!("[version_purger] No max_age_days configured; skipping age-based pruning");
        }

        // Prune by count (preserve latest N per entity/workflow/definition)
        if let Some(keep) = max_versions {
            info!(
                "[version_purger] Pruning versions, keeping latest {} per item across all repositories",
                keep
            );
            for repo in &repositories {
                let repo_name = repo.repository_name();
                match repo.prune_keep_latest(keep).await {
                    Ok(count) => {
                        info!(
                            "[version_purger] Pruned {} versions, kept latest {} per item from {}",
                            count, keep, repo_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "[version_purger] Failed to prune versions by count from {}: {}",
                            repo_name, e
                        );
                        return Err(e);
                    }
                }
            }
        } else {
            info!("[version_purger] No max_versions configured; skipping count-based pruning");
        }

        info!("[version_purger] Version purging task completed successfully");
        Ok(())
    }
}
