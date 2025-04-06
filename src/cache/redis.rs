use async_trait::async_trait;
use redis::{Client, AsyncCommands};
use serde::{Serialize, de::DeserializeOwned};

use crate::error::{Error, Result};
use super::CacheBackend;

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
        let mut conn = client.get_async_connection().await
            .map_err(|e| Error::Cache(format!("Failed to get Redis connection: {}", e)))?;
            
        redis::cmd("PING").query_async(&mut conn).await
            .map_err(|e| Error::Cache(format!("Failed to ping Redis: {}", e)))?;
            
        Ok(Self {
            client,
            default_ttl,
        })
    }
}

#[async_trait]
impl CacheBackend for RedisCache {
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Cache(format!("Failed to get Redis connection: {}", e)))?;
            
        let result: Option<String> = conn.get(key).await
            .map_err(|e| Error::Cache(format!("Failed to get value from Redis: {}", e)))?;
            
        match result {
            Some(data) => {
                // Deserialize the value
                match serde_json::from_str::<T>(&data) {
                    Ok(value) => Ok(Some(value)),
                    Err(e) => Err(Error::Serialization(e)),
                }
            },
            None => Ok(None),
        }
    }
    
    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T, ttl: Option<u64>) -> Result<()> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Cache(format!("Failed to get Redis connection: {}", e)))?;
            
        // Serialize the value
        let serialized = serde_json::to_string(value)
            .map_err(Error::Serialization)?;
            
        // Set with expiration
        let ttl = ttl.unwrap_or(self.default_ttl);
        
        conn.set_ex(key, serialized, ttl).await
            .map_err(|e| Error::Cache(e.to_string()))?;
        
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Cache(format!("Failed to get Redis connection: {}", e)))?;
            
        conn.del(key).await
            .map_err(|e| Error::Cache(format!("Failed to delete value from Redis: {}", e)))?;
            
        Ok(())
    }
    
    async fn clear(&self) -> Result<()> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| Error::Cache(format!("Failed to get Redis connection: {}", e)))?;
            
        redis::cmd("FLUSHDB").query_async(&mut conn).await
            .map_err(|e| Error::Cache(format!("Failed to clear Redis: {}", e)))?;
            
        Ok(())
    }
} 