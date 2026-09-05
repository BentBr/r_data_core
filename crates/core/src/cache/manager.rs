#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use crate::cache::backend::CacheBackend;
use crate::cache::in_memory::InMemoryCache;
use crate::cache::redis::RedisCache;
use crate::config::CacheConfig;
use crate::error::Result;

/// Cache manager that handles multiple cache backends
pub struct CacheManager {
    config: CacheConfig,
    in_memory: Arc<InMemoryCache>,
    redis: Option<Arc<RedisCache>>,
}

impl CacheManager {
    /// Create a new cache manager with the given configuration
    ///
    /// # Arguments
    /// * `config` - Cache configuration
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        let max_size = config.max_size.try_into().unwrap_or(10000);
        let in_memory = Arc::new(InMemoryCache::new(config.ttl, max_size));

        Self {
            config,
            in_memory,
            redis: None,
        }
    }

    /// Add a Redis cache backend
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL
    ///
    /// # Errors
    /// Returns an error if Redis connection fails
    pub async fn with_redis(mut self, redis_url: &str) -> Result<Self> {
        if redis_url.is_empty() {
            return Ok(self);
        }

        let redis_cache = RedisCache::new(redis_url, self.config.ttl).await?;
        self.redis = Some(Arc::new(redis_cache));

        Ok(self)
    }

    /// Get a value from the cache
    ///
    /// # Errors
    /// Returns an error if cache retrieval fails
    pub async fn get<T: serde::de::DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>> {
        if !self.config.enabled {
            return Ok(None);
        }

        // Try Redis first if available
        if let Some(redis) = &self.redis {
            match redis.get::<T>(key).await {
                Ok(Some(value)) => return Ok(Some(value)),
                Ok(None) => {}
                Err(e) => {
                    log::warn!("Redis cache error: {e}");
                    // Continue to in-memory cache
                }
            }
        }

        // Try in-memory cache
        self.in_memory.get::<T>(key).await
    }

    /// Set a value in the cache
    ///
    /// # Errors
    /// Returns an error if cache storage fails
    pub async fn set<T: serde::Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<u64>,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let ttl = ttl.unwrap_or(self.config.ttl);

        // Set in Redis if available
        if let Some(redis) = &self.redis {
            // Ignore Redis errors, just log them
            if let Err(e) = redis.set::<T>(key, value, Some(ttl)).await {
                log::warn!("Redis cache error: {e}");
            }
        }

        // Always set in in-memory cache
        self.in_memory.set::<T>(key, value, Some(ttl)).await
    }

    /// Atomically increment a counter and return the new value.
    ///
    /// Redis is authoritative when configured so the counter is shared across
    /// instances; otherwise the in-memory backend is used. Returns `0` when the
    /// cache is disabled, which callers read as "no attempts recorded".
    ///
    /// # Errors
    /// Returns an error if the counter cannot be updated.
    pub async fn increment(&self, key: &str, ttl: u64) -> Result<u32> {
        if !self.config.enabled {
            return Ok(0);
        }

        if let Some(redis) = &self.redis {
            match redis.increment(key, ttl).await {
                Ok(count) => return Ok(count),
                Err(e) => log::warn!("Redis cache error: {e}"),
            }
        }

        self.in_memory.increment(key, ttl).await
    }

    /// Delete a value from the cache
    ///
    /// # Errors
    /// Returns an error if cache deletion fails
    pub async fn delete(&self, key: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Delete from Redis if available
        if let Some(redis) = &self.redis {
            // Ignore Redis errors, just log them
            if let Err(e) = redis.delete(key).await {
                log::warn!("Redis cache error: {e}");
            }
        }

        // Always delete from in-memory cache
        self.in_memory.delete(key).await
    }

    /// Clear the entire cache
    ///
    /// # Errors
    /// Returns an error if cache clearing fails
    pub async fn clear(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Clear Redis if available
        if let Some(redis) = &self.redis {
            // Ignore Redis errors, just log them
            if let Err(e) = redis.clear().await {
                log::warn!("Redis cache error: {e}");
            }
        }

        // Always clear in-memory cache
        self.in_memory.clear().await
    }

    /// Delete all cache entries matching a prefix
    ///
    /// This is useful for clearing specific cache types (e.g., all `entity_definitions`)
    ///
    /// # Arguments
    /// * `prefix` - Key prefix to match
    ///
    /// # Returns
    /// The number of entries deleted
    ///
    /// # Errors
    /// Returns an error if cache deletion fails
    pub async fn delete_by_prefix(&self, prefix: &str) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let mut deleted_count = 0;

        // Delete from Redis if available
        if let Some(redis) = &self.redis {
            match redis.delete_by_prefix(prefix).await {
                Ok(count) => deleted_count = count,
                Err(e) => {
                    log::warn!("Redis cache error during prefix deletion: {e}");
                }
            }
        }

        // Delete from in-memory cache
        match self.in_memory.delete_by_prefix(prefix).await {
            Ok(count) => {
                // Both backends store the same keys, so report the higher count
                // rather than summing (which would double-count)
                deleted_count = deleted_count.max(count);
            }
            Err(e) => {
                log::warn!("In-memory cache error during prefix deletion: {e}");
            }
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheConfig, CacheManager};

    fn manager(enabled: bool) -> CacheManager {
        CacheManager::new(CacheConfig {
            entity_definition_ttl: 0,
            api_key_ttl: 600,
            enabled,
            ttl: 3600,
            max_size: 100,
        })
    }

    #[tokio::test]
    async fn increment_counts_up_and_is_readable_via_get() {
        let cache = manager(true);

        assert_eq!(cache.increment("rl:1", 60).await.unwrap_or(0), 1);
        assert_eq!(cache.increment("rl:1", 60).await.unwrap_or(0), 2);
        assert_eq!(cache.increment("rl:1", 60).await.unwrap_or(0), 3);
        assert_eq!(cache.get::<u32>("rl:1").await.unwrap_or(None), Some(3));
    }

    #[tokio::test]
    async fn increment_keys_are_independent() {
        let cache = manager(true);

        cache.increment("rl:a", 60).await.unwrap_or(0);
        cache.increment("rl:a", 60).await.unwrap_or(0);

        assert_eq!(cache.increment("rl:b", 60).await.unwrap_or(0), 1);
    }

    #[tokio::test]
    async fn delete_restarts_the_window() {
        let cache = manager(true);

        cache.increment("rl:c", 60).await.unwrap_or(0);
        cache.increment("rl:c", 60).await.unwrap_or(0);
        cache.delete("rl:c").await.unwrap_or(());

        assert_eq!(cache.increment("rl:c", 60).await.unwrap_or(0), 1);
    }

    /// With the cache off there is nothing to count, and callers read `0` as
    /// "no attempts recorded" rather than tripping the limit.
    #[tokio::test]
    async fn disabled_cache_never_reports_attempts() {
        let cache = manager(false);

        assert_eq!(cache.increment("rl:d", 60).await.unwrap_or(99), 0);
        assert_eq!(cache.increment("rl:d", 60).await.unwrap_or(99), 0);
        assert_eq!(cache.get::<u32>("rl:d").await.unwrap_or(None), None);
    }

    /// Concurrent increments must not lose updates — the read-then-write version
    /// of this counter let parallel requests slip past the limit.
    #[tokio::test]
    async fn concurrent_increments_do_not_lose_updates() {
        let cache = std::sync::Arc::new(manager(true));

        let mut handles = Vec::new();
        for _ in 0..50 {
            let cache = cache.clone();
            handles.push(tokio::spawn(async move {
                cache.increment("rl:race", 60).await.unwrap_or(0)
            }));
        }
        for handle in handles {
            let _ = handle.await;
        }

        assert_eq!(cache.get::<u32>("rl:race").await.unwrap_or(None), Some(50));
    }
}
