use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

pub mod in_memory;
pub mod redis;

use crate::error::Result;

use crate::config::CacheConfig;

use self::in_memory::InMemoryCache;
use self::redis::RedisCache;

/// Trait for cache backends
#[async_trait]
pub trait CacheBackend: Send + Sync {
    /// Get a value from the cache
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>>;

    /// Set a value in the cache with an optional expiration time
    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<u64>,
    ) -> Result<()>;

    /// Delete a value from the cache
    async fn delete(&self, key: &str) -> Result<()>;

    /// Clear the entire cache
    async fn clear(&self) -> Result<()>;

    /// Delete all cache entries matching a prefix
    /// Returns the number of entries deleted
    async fn delete_by_prefix(&self, prefix: &str) -> Result<usize>;
}

/// Cache manager that handles multiple cache backends
pub struct CacheManager {
    config: CacheConfig,
    in_memory: Arc<InMemoryCache>,
    redis: Option<Arc<RedisCache>>,
}

impl CacheManager {
    /// Create a new cache manager with the given configuration
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
    pub async fn with_redis(mut self, redis_url: &str) -> Result<Self> {
        if redis_url.is_empty() {
            return Ok(self);
        }

        let redis_cache = RedisCache::new(redis_url, self.config.ttl).await?;
        self.redis = Some(Arc::new(redis_cache));

        Ok(self)
    }

    /// Get a value from the cache
    pub async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
        if !self.config.enabled {
            return Ok(None);
        }

        // Try Redis first if available
        if let Some(redis) = &self.redis {
            match redis.get::<T>(key).await {
                Ok(Some(value)) => return Ok(Some(value)),
                Ok(None) => {}
                Err(e) => {
                    log::warn!("Redis cache error: {}", e);
                    // Continue to in-memory cache
                }
            }
        }

        // Try in-memory cache
        self.in_memory.get::<T>(key).await
    }

    /// Set a value in the cache
    pub async fn set<T: Serialize + Send + Sync>(
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
                log::warn!("Redis cache error: {}", e);
            }
        }

        // Always set in in-memory cache
        self.in_memory.set::<T>(key, value, Some(ttl)).await
    }

    /// Delete a value from the cache
    pub async fn delete(&self, key: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Delete from Redis if available
        if let Some(redis) = &self.redis {
            // Ignore Redis errors, just log them
            if let Err(e) = redis.delete(key).await {
                log::warn!("Redis cache error: {}", e);
            }
        }

        // Always delete from in-memory cache
        self.in_memory.delete(key).await
    }

    /// Clear the entire cache
    pub async fn clear(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Clear Redis if available
        if let Some(redis) = &self.redis {
            // Ignore Redis errors, just log them
            if let Err(e) = redis.clear().await {
                log::warn!("Redis cache error: {}", e);
            }
        }

        // Always clear in-memory cache
        self.in_memory.clear().await
    }

    /// Delete all cache entries matching a prefix
    /// This is useful for clearing specific cache types (e.g., all entity_definitions)
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
                    log::warn!("Redis cache error during prefix deletion: {}", e);
                }
            }
        }

        // Delete from in-memory cache
        match self.in_memory.delete_by_prefix(prefix).await {
            Ok(count) => deleted_count += count,
            Err(e) => {
                log::warn!("In-memory cache error during prefix deletion: {}", e);
            }
        }

        Ok(deleted_count)
    }
}
