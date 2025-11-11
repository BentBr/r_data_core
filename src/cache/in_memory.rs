use async_trait::async_trait;
use lru::LruCache;
use serde::{de::DeserializeOwned, Serialize};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::CacheBackend;
use crate::error::{Error, Result};

/// Cache entry with value and expiration time
struct CacheEntry {
    /// The serialized value
    value: Vec<u8>,
    /// When this entry expires
    expires_at: Option<Instant>,
}

/// In-memory cache implementation
pub struct InMemoryCache {
    /// Cache data with expiration tracking
    data: RwLock<LruCache<String, CacheEntry>>,
    /// Default TTL in seconds
    default_ttl: u64,
}

impl InMemoryCache {
    /// Create a new in-memory cache
    pub fn new(default_ttl: u64, max_size: usize) -> Self {
        let capacity = NonZeroUsize::new(max_size).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            data: RwLock::new(LruCache::new(capacity)),
            default_ttl,
        }
    }

    /// Check if an entry is expired
    fn is_expired(entry: &CacheEntry) -> bool {
        if let Some(expires_at) = entry.expires_at {
            Instant::now() > expires_at
        } else {
            false
        }
    }
}

#[async_trait]
impl CacheBackend for InMemoryCache {
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
        let mut cache = self.data.write().await;

        if let Some(entry) = cache.get(key) {
            if Self::is_expired(entry) {
                // Remove expired entry
                cache.pop(key);
                return Ok(None);
            }

            // Deserialize the value
            match serde_json::from_slice::<T>(&entry.value) {
                Ok(value) => Ok(Some(value)),
                Err(e) => Err(Error::Serialization(e)),
            }
        } else {
            Ok(None)
        }
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
        let mut cache = self.data.write().await;
        cache.put(key.to_string(), entry);

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut cache = self.data.write().await;
        cache.pop(key);
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let mut cache = self.data.write().await;
        cache.clear();
        Ok(())
    }

    async fn delete_by_prefix(&self, prefix: &str) -> Result<usize> {
        let mut cache = self.data.write().await;
        let mut deleted = 0;

        // Collect keys to delete (we can't modify while iterating)
        let keys_to_delete: Vec<String> = cache
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(key, _)| key.clone())
            .collect();

        // Delete the keys
        for key in keys_to_delete {
            if cache.pop(&key).is_some() {
                deleted += 1;
            }
        }

        Ok(deleted)
    }
}
