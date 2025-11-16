use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityVersioningSettings {
    pub enabled: bool,
    pub max_versions: Option<i32>,
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
