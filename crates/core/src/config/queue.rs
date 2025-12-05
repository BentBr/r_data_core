#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Redis connection URL
    pub redis_url: String,
    /// Redis key for fetch jobs
    pub fetch_key: String,
    /// Redis key for process jobs
    pub process_key: String,
}
