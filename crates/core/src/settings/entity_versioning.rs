#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

/// Entity versioning configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityVersioningSettings {
    /// Whether entity versioning is enabled
    pub enabled: bool,
    /// Maximum number of versions to keep per entity (None = unlimited)
    pub max_versions: Option<i32>,
    /// Maximum age in days for versions (None = no age limit)
    pub max_age_days: Option<i32>,
}

impl Default for EntityVersioningSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_versions: None,
            max_age_days: Some(180),
        }
    }
}
