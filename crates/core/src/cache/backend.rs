#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::Result;

/// Trait for cache backends
#[async_trait]
pub trait CacheBackend: Send + Sync {
    /// Get a value from the cache
    ///
    /// # Errors
    /// Returns an error if cache retrieval fails
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>>;

    /// Set a value in the cache with an optional expiration time
    ///
    /// # Arguments
    /// * `key` - Cache key
    /// * `value` - Value to cache
    /// * `ttl` - Time to live in seconds (None = no expiration)
    ///
    /// # Errors
    /// Returns an error if cache storage fails
    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<u64>,
    ) -> Result<()>;

    /// Delete a value from the cache
    ///
    /// # Errors
    /// Returns an error if cache deletion fails
    async fn delete(&self, key: &str) -> Result<()>;

    /// Clear the entire cache
    ///
    /// # Errors
    /// Returns an error if cache clearing fails
    async fn clear(&self) -> Result<()>;

    /// Delete all cache entries matching a prefix
    ///
    /// # Arguments
    /// * `prefix` - Key prefix to match
    ///
    /// # Returns
    /// The number of entries deleted
    ///
    /// # Errors
    /// Returns an error if cache deletion fails
    async fn delete_by_prefix(&self, prefix: &str) -> Result<usize>;
}

