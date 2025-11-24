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
    pub async fn get<T: serde::de::DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
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
                Ok(count) => deleted_count += count,
                Err(e) => {
                    log::warn!("Redis cache error during prefix deletion: {e}");
                }
            }
        }

        // Delete from in-memory cache
        match self.in_memory.delete_by_prefix(prefix).await {
            Ok(count) => deleted_count += count,
            Err(e) => {
                log::warn!("In-memory cache error during prefix deletion: {e}");
            }
        }

        Ok(deleted_count)
    }
}

