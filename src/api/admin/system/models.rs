use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityVersioningSettingsDto {
    pub enabled: bool,
    pub max_versions: Option<i32>,
    pub max_age_days: Option<i32>,
}
