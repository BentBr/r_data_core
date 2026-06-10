#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

/// Outbox configuration settings for workflow fetch and push paths.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutboxSettings {
    /// Whether workflow fetch dispatch should use outbox indirection.
    pub fetch_enabled: bool,
    /// Whether workflow push delivery should use outbox indirection.
    pub push_enabled: bool,
}
