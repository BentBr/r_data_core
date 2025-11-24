#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,

    /// Cache time-to-live in seconds (default TTL)
    pub ttl: u64,

    /// Maximum cache size (number of items)
    pub max_size: u64,

    /// TTL for entity definitions cache (0 = no expiration, use None when setting)
    pub entity_definition_ttl: u64,

    /// TTL for API keys cache in seconds
    pub api_key_ttl: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: 3600,
            max_size: 10000,
            entity_definition_ttl: 0,
            api_key_ttl: 600,
        }
    }
}

