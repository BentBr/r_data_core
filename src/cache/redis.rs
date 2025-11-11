use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};

use super::CacheBackend;
use crate::error::{Error, Result};

/// Redis cache implementation
pub struct RedisCache {
    /// Redis client
    client: Client,
    /// Default TTL in seconds
    default_ttl: u64,
}

impl RedisCache {
    /// Create a new Redis cache
    pub async fn new(redis_url: &str, default_ttl: u64) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| Error::Cache(format!("Failed to connect to Redis: {}", e)))?;

        // Test connection
        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::Cache(format!("Failed to get Redis connection: {}", e)))?;

        redis::cmd("PING")
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| Error::Cache(format!("Failed to ping Redis: {}", e)))?;

        Ok(Self {
            client,
            default_ttl,
        })
    }

    async fn get_connection(&self) -> Result<MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::Cache(format!("Failed to get Redis connection: {}", e)))
    }
}

#[async_trait]
impl CacheBackend for RedisCache {
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.get_connection().await?;

        let result: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| Error::Cache(format!("Failed to get value from Redis: {}", e)))?;

        match result {
            Some(data) => {
                // Deserialize the value
                match serde_json::from_str::<T>(&data) {
                    Ok(value) => Ok(Some(value)),
                    Err(e) => Err(Error::Serialization(e)),
                }
            }
            None => Ok(None),
        }
    }

    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<u64>,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // Serialize the value
        let serialized = serde_json::to_string(value).map_err(Error::Serialization)?;

        // Set with expiration
        let ttl = ttl.unwrap_or(self.default_ttl);

        conn.set_ex::<_, _, ()>(key, serialized, ttl)
            .await
            .map_err(|e| Error::Cache(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;

        conn.del::<_, ()>(key)
            .await
            .map_err(|e| Error::Cache(format!("Failed to delete value from Redis: {}", e)))?;

        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;

        redis::cmd("FLUSHDB")
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| Error::Cache(format!("Failed to clear Redis: {}", e)))?;

        Ok(())
    }

    async fn delete_by_prefix(&self, prefix: &str) -> Result<usize> {
        let mut conn = self.get_connection().await?;
        let mut deleted = 0;
        let mut cursor = 0u64;
        let pattern = format!("{}*", prefix);

        loop {
            // Use SCAN to find keys matching the pattern
            let result: (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| Error::Cache(format!("Failed to scan Redis keys: {}", e)))?;

            cursor = result.0;
            let keys = result.1;

            if !keys.is_empty() {
                // Delete the keys
                let count: u64 = redis::cmd("DEL")
                    .arg(&keys)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| Error::Cache(format!("Failed to delete Redis keys: {}", e)))?;
                deleted += count as usize;
            }

            // If cursor is 0, we've scanned all keys
            if cursor == 0 {
                break;
            }
        }

        Ok(deleted)
    }
}
