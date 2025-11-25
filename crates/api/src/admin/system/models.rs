#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use r_data_core_core::settings::EntityVersioningSettings;

/// DTO for entity versioning settings (API layer wrapper)
///
/// This is a thin wrapper around the core `EntityVersioningSettings` type
/// to add OpenAPI schema generation support.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityVersioningSettingsDto {
    /// Whether entity versioning is enabled
    pub enabled: bool,
    /// Maximum number of versions to keep per entity
    pub max_versions: Option<i32>,
    /// Maximum age in days for versions
    pub max_age_days: Option<i32>,
}

impl From<EntityVersioningSettings> for EntityVersioningSettingsDto {
    fn from(settings: EntityVersioningSettings) -> Self {
        Self {
            enabled: settings.enabled,
            max_versions: settings.max_versions,
            max_age_days: settings.max_age_days,
        }
    }
}

impl From<EntityVersioningSettingsDto> for EntityVersioningSettings {
    fn from(dto: EntityVersioningSettingsDto) -> Self {
        Self {
            enabled: dto.enabled,
            max_versions: dto.max_versions,
            max_age_days: dto.max_age_days,
        }
    }
}

/// Request body for updating settings
#[derive(Deserialize, Serialize, ToSchema)]
pub struct UpdateSettingsBody {
    /// Whether versioning is enabled
    pub enabled: Option<bool>,
    /// Maximum number of versions to keep
    pub max_versions: Option<i32>,
    /// Maximum age in days
    pub max_age_days: Option<i32>,
}

