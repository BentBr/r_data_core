#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Cache service for managing Redis and in-memory cache operations.
//!
//! This service provides high-level cache management operations,
//! wrapping the underlying `CacheManager` with CLI-friendly functionality.

use std::sync::Arc;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;

/// Result of a cache clear operation
#[derive(Debug, Clone)]
pub struct CacheClearResult {
    /// Number of keys deleted
    pub deleted_count: usize,
    /// Whether the operation was a dry run
    pub dry_run: bool,
    /// Keys that were deleted (or would be deleted in dry-run mode)
    pub keys: Vec<String>,
}

/// Cache service for managing cache operations
pub struct CacheService {
    manager: Arc<CacheManager>,
}

impl CacheService {
    /// Create a new cache service
    #[must_use]
    pub const fn new(manager: Arc<CacheManager>) -> Self {
        Self { manager }
    }

    /// Create a cache service with Redis support
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL
    /// * `default_ttl` - Default TTL for cache entries in seconds
    ///
    /// # Errors
    /// Returns an error if Redis connection fails
    pub async fn with_redis(redis_url: &str, default_ttl: u64) -> Result<Self> {
        let config = CacheConfig {
            enabled: true,
            ttl: default_ttl,
            max_size: 10000,
            entity_definition_ttl: 0,
            api_key_ttl: 600,
        };

        let manager = CacheManager::new(config).with_redis(redis_url).await?;
        Ok(Self {
            manager: Arc::new(manager),
        })
    }

    /// Clear the entire cache
    ///
    /// # Errors
    /// Returns an error if cache clearing fails
    pub async fn clear_all(&self) -> Result<()> {
        self.manager.clear().await
    }

    /// Clear cache entries matching a prefix
    ///
    /// # Arguments
    /// * `prefix` - Key prefix to match
    ///
    /// # Returns
    /// The number of entries deleted
    ///
    /// # Errors
    /// Returns an error if cache deletion fails
    pub async fn clear_by_prefix(&self, prefix: &str) -> Result<usize> {
        self.manager.delete_by_prefix(prefix).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_data_core_core::config::CacheConfig;

    fn create_test_config() -> CacheConfig {
        CacheConfig {
            enabled: true,
            ttl: 300,
            max_size: 1000,
            entity_definition_ttl: 0,
            api_key_ttl: 600,
        }
    }

    #[tokio::test]
    async fn test_cache_service_creation() {
        let config = create_test_config();
        let manager = Arc::new(CacheManager::new(config));
        let service = CacheService::new(manager);

        // Should be able to clear without error (even with empty cache)
        let result = service.clear_all().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_clear_all_with_in_memory() {
        let config = create_test_config();
        let manager = Arc::new(CacheManager::new(config));
        let service = CacheService::new(manager.clone());

        // Set some values
        manager.set("key1", &"value1", None).await.unwrap();
        manager.set("key2", &"value2", None).await.unwrap();

        // Clear all
        service.clear_all().await.unwrap();

        // Values should be gone
        let result: Option<String> = manager.get("key1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_clear_by_prefix_with_in_memory() {
        let config = create_test_config();
        let manager = Arc::new(CacheManager::new(config));
        let service = CacheService::new(manager.clone());

        // Set some values with different prefixes
        manager
            .set("entity_definitions:1", &"def1", None)
            .await
            .unwrap();
        manager
            .set("entity_definitions:2", &"def2", None)
            .await
            .unwrap();
        manager.set("api_keys:1", &"key1", None).await.unwrap();

        // Clear by prefix
        let deleted = service
            .clear_by_prefix("entity_definitions:")
            .await
            .unwrap();
        assert_eq!(deleted, 2);

        // entity_definitions should be gone
        let result: Option<String> = manager.get("entity_definitions:1").await.unwrap();
        assert!(result.is_none());

        // api_keys should still exist
        let result: Option<String> = manager.get("api_keys:1").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_clear_by_prefix_no_matches() {
        let config = create_test_config();
        let manager = Arc::new(CacheManager::new(config));
        let service = CacheService::new(manager.clone());

        // Set some values
        manager.set("api_keys:1", &"key1", None).await.unwrap();

        // Clear by non-matching prefix
        let deleted = service.clear_by_prefix("nonexistent:").await.unwrap();
        assert_eq!(deleted, 0);

        // api_keys should still exist
        let result: Option<String> = manager.get("api_keys:1").await.unwrap();
        assert!(result.is_some());
    }
}
