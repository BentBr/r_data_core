#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use lru::LruCache;
use serde::{de::DeserializeOwned, Serialize};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::cache::backend::CacheBackend;
use crate::error::{Error, Result};

/// Cache entry with value and expiration time
struct CacheEntry {
    /// The serialized value
    value: Vec<u8>,
    /// When this entry expires
    expires_at: Option<Instant>,
}

/// In-memory cache implementation using LRU eviction
pub struct InMemoryCache {
    /// Cache data with expiration tracking
    data: RwLock<LruCache<String, CacheEntry>>,
    /// Default TTL in seconds
    default_ttl: u64,
}

impl InMemoryCache {
    /// Create a new in-memory cache
    ///
    /// # Arguments
    /// * `default_ttl` - Default time-to-live in seconds
    /// * `max_size` - Maximum number of cache entries
    ///
    /// # Panics
    /// Panics if `max_size` is 0 and `NonZeroUsize::new(1)` fails (should never happen)
    #[must_use]
    pub fn new(default_ttl: u64, max_size: usize) -> Self {
        let capacity = NonZeroUsize::new(max_size).unwrap_or(NonZeroUsize::MIN);
        Self {
            data: RwLock::new(LruCache::new(capacity)),
            default_ttl,
        }
    }

    /// Check if an entry is expired
    fn is_expired(entry: &CacheEntry) -> bool {
        entry
            .expires_at
            .is_some_and(|expires_at| Instant::now() > expires_at)
    }
}

#[async_trait]
impl CacheBackend for InMemoryCache {
    #[allow(clippy::significant_drop_tightening)]
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
        // We need to hold the lock while checking expiration to avoid race conditions
        let value = {
            let mut cache = self.data.write().await;

            if let Some(entry) = cache.get(key) {
                if Self::is_expired(entry) {
                    // Remove expired entry
                    cache.pop(key);
                    return Ok(None);
                }

                // Clone the value for deserialization outside the lock
                Some(entry.value.clone())
            } else {
                None
            }
        };

        // Deserialize outside the lock
        value.map_or_else(
            || Ok(None),
            |serialized| {
                serde_json::from_slice::<T>(&serialized)
                    .map(Some)
                    .map_err(Error::Serialization)
            },
        )
    }

    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<u64>,
    ) -> Result<()> {
        // Serialize the value
        let serialized = serde_json::to_string(value).map_err(Error::Serialization)?;

        // Calculate expiration time
        let ttl = ttl.unwrap_or(self.default_ttl);
        let expires_at = if ttl > 0 {
            Some(Instant::now() + Duration::from_secs(ttl))
        } else {
            None
        };

        // Create cache entry
        let entry = CacheEntry {
            value: serialized.into_bytes(),
            expires_at,
        };

        // Store in cache
        {
            #[allow(clippy::significant_drop_tightening)]
            let mut cache = self.data.write().await;
            cache.put(key.to_string(), entry);
        }

        Ok(())
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn increment(&self, key: &str, ttl: u64) -> Result<u32> {
        let mut cache = self.data.write().await;

        // Keep the existing expiry so the window does not slide forward.
        let existing = cache.get(key).filter(|e| !Self::is_expired(e));
        let expires_at = existing.and_then(|e| e.expires_at);
        let current = existing
            .and_then(|e| serde_json::from_slice::<u32>(&e.value).ok())
            .unwrap_or(0);

        let next = current.saturating_add(1);
        let expires_at =
            expires_at.or_else(|| (ttl > 0).then(|| Instant::now() + Duration::from_secs(ttl)));

        cache.put(
            key.to_string(),
            CacheEntry {
                value: next.to_string().into_bytes(),
                expires_at,
            },
        );

        Ok(next)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        {
            let mut cache = self.data.write().await;
            cache.pop(key);
        }
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        {
            let mut cache = self.data.write().await;
            cache.clear();
        }
        Ok(())
    }

    async fn delete_by_prefix(&self, prefix: &str) -> Result<usize> {
        let keys_to_delete = {
            let cache = self.data.read().await;
            // Collect keys to delete (we can't modify while iterating)
            cache
                .iter()
                .filter(|(key, _)| key.starts_with(prefix))
                .map(|(key, _)| key.clone())
                .collect::<Vec<String>>()
        };

        // Delete the keys
        let mut deleted = 0;
        {
            let mut cache = self.data.write().await;
            for key in keys_to_delete {
                if cache.pop(&key).is_some() {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheBackend, InMemoryCache};

    #[tokio::test]
    async fn increment_counts_up_from_absent() {
        let cache = InMemoryCache::new(60, 16);

        assert_eq!(cache.increment("k", 60).await.unwrap_or(0), 1);
        assert_eq!(cache.increment("k", 60).await.unwrap_or(0), 2);
        assert_eq!(cache.get::<u32>("k").await.unwrap_or(None), Some(2));
    }

    #[tokio::test]
    async fn increment_keeps_the_original_window() {
        let cache = InMemoryCache::new(60, 16);

        cache.increment("k", 60).await.unwrap_or(0);
        let first_expiry = {
            let mut data = cache.data.write().await;
            data.get("k").and_then(|e| e.expires_at)
        };

        cache.increment("k", 600).await.unwrap_or(0);
        let second_expiry = {
            let mut data = cache.data.write().await;
            data.get("k").and_then(|e| e.expires_at)
        };

        assert_eq!(first_expiry, second_expiry);
    }

    #[tokio::test]
    async fn increment_restarts_after_delete() {
        let cache = InMemoryCache::new(60, 16);

        cache.increment("k", 60).await.unwrap_or(0);
        cache.delete("k").await.unwrap_or(());

        assert_eq!(cache.increment("k", 60).await.unwrap_or(0), 1);
    }

    #[tokio::test]
    async fn increment_reads_a_value_written_by_set() {
        let cache = InMemoryCache::new(60, 16);
        cache.set("k", &7u32, Some(60)).await.unwrap_or(());

        assert_eq!(cache.increment("k", 60).await.unwrap_or(0), 8);
    }
}
