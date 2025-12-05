use async_trait::async_trait;

use crate::error::Result;

/// Trait for version repositories that support purging operations
#[async_trait]
pub trait VersionPurger: Send + Sync {
    /// Get the name of the repository type (for logging)
    fn repository_name(&self) -> &'static str;

    /// Prune versions older than the specified number of days
    async fn prune_older_than_days(&self, days: i32) -> Result<u64>;

    /// Prune versions, keeping only the latest N versions per entity/workflow/definition
    async fn prune_keep_latest(&self, keep: i32) -> Result<u64>;
}
