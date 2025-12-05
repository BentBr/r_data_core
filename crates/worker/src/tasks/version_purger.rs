#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use log::{info, warn};

use r_data_core_core::cache::CacheManager;
use r_data_core_core::maintenance::task::TaskContext;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_core::versioning::purger_trait::VersionPurger;
use r_data_core_persistence::{
    EntityDefinitionVersioningRepository, VersionRepository, WorkflowVersioningRepository,
};
use r_data_core_services::SettingsService;

/// Maintenance task that purges old entity versions based on configured settings
pub struct VersionPurgerTask {
    cron: String,
}

impl VersionPurgerTask {
    /// Create a new `VersionPurgerTask` with the given cron expression
    ///
    /// # Arguments
    /// * `version_purger_cron` - Cron expression for scheduling this task
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // String is not const-constructible
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

    async fn execute(
        &self,
        context: &dyn TaskContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("[version_purger] Starting version purging task");

        let pool = context.pool();

        // Get settings service
        // TODO: Move SettingsService to appropriate crate
        let cache_manager = CacheManager::new(r_data_core_core::config::CacheConfig {
            entity_definition_ttl: 3600,
            api_key_ttl: 600,
            enabled: true,
            ttl: 3600,
            max_size: 10000,
        });
        let settings_service =
            SettingsService::new(pool.clone(), std::sync::Arc::new(cache_manager));

        let settings = match settings_service.get_entity_versioning_settings().await {
            Ok(s) => s,
            Err(e) => {
                warn!("[version_purger] Failed to retrieve entity versioning settings: {e}");
                return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            }
        };

        let r_data_core_core::settings::EntityVersioningSettings {
            enabled,
            max_versions,
            max_age_days,
        } = settings;

        info!(
            "[version_purger] Entity versioning settings - enabled: {enabled}, max_versions: {max_versions:?}, max_age_days: {max_age_days:?}"
        );

        if !enabled {
            info!("[version_purger] Entity versioning disabled; prune skipped");
            return Ok(());
        }

        // Initialize all three version repositories
        let entity_repo = VersionRepository::new(pool.clone());
        let workflow_repo = WorkflowVersioningRepository::new(pool.clone());
        let definition_repo = EntityDefinitionVersioningRepository::new(pool.clone());

        let repositories: Vec<Box<dyn VersionPurger>> = vec![
            Box::new(entity_repo),
            Box::new(workflow_repo),
            Box::new(definition_repo),
        ];

        // Prune by age first if configured
        if let Some(days) = max_age_days {
            info!(
                "[version_purger] Pruning versions older than {days} days across all repositories"
            );
            for repo in &repositories {
                let repo_name = repo.repository_name();
                match repo.prune_older_than_days(days).await {
                    Ok(count) => {
                        info!(
                            "[version_purger] Pruned {count} versions older than {days} days from {repo_name}"
                        );
                    }
                    Err(e) => {
                        warn!(
                            "[version_purger] Failed to prune versions by age from {repo_name}: {e}"
                        );
                        return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
                    }
                }
            }
        } else {
            info!("[version_purger] No max_age_days configured; skipping age-based pruning");
        }

        // Prune by count (preserve latest N per entity/workflow/definition)
        if let Some(keep) = max_versions {
            info!(
                "[version_purger] Pruning versions, keeping latest {keep} per item across all repositories"
            );
            for repo in &repositories {
                let repo_name = repo.repository_name();
                match repo.prune_keep_latest(keep).await {
                    Ok(count) => {
                        info!(
                            "[version_purger] Pruned {count} versions, kept latest {keep} per item from {repo_name}"
                        );
                    }
                    Err(e) => {
                        warn!(
                            "[version_purger] Failed to prune versions by count from {repo_name}: {e}"
                        );
                        return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
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
