#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use r_data_core_core::settings::{EntityVersioningSettings, WorkflowRunLogSettings};

/// DTO for entity versioning settings (API layer wrapper)
///
/// This is a thin wrapper around the core `EntityVersioningSettings` type
/// to add `OpenAPI` schema generation support.
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

/// DTO for workflow run log settings (API layer wrapper)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowRunLogSettingsDto {
    /// Whether workflow run logs pruning is enabled
    pub enabled: bool,
    /// Maximum number of runs to keep per workflow
    pub max_runs: Option<i32>,
    /// Maximum age in days for workflow runs
    pub max_age_days: Option<i32>,
}

impl From<WorkflowRunLogSettings> for WorkflowRunLogSettingsDto {
    fn from(settings: WorkflowRunLogSettings) -> Self {
        Self {
            enabled: settings.enabled,
            max_runs: settings.max_runs,
            max_age_days: settings.max_age_days,
        }
    }
}

impl From<WorkflowRunLogSettingsDto> for WorkflowRunLogSettings {
    fn from(dto: WorkflowRunLogSettingsDto) -> Self {
        Self {
            enabled: dto.enabled,
            max_runs: dto.max_runs,
            max_age_days: dto.max_age_days,
        }
    }
}

/// Request body for updating workflow run log settings
#[derive(Deserialize, Serialize, ToSchema)]
pub struct UpdateWorkflowRunLogSettingsBody {
    /// Whether pruning is enabled
    pub enabled: Option<bool>,
    /// Maximum number of runs to keep per workflow
    pub max_runs: Option<i32>,
    /// Maximum age in days
    pub max_age_days: Option<i32>,
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

/// License state enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LicenseStateDto {
    /// No license key provided
    None,
    /// License key is invalid
    Invalid,
    /// Network/technical error during verification
    Error,
    /// License key is valid
    Valid,
}

/// DTO for license status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LicenseStatusDto {
    /// License state
    pub state: LicenseStateDto,
    /// Company name (if license is present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<String>,
    /// License type (if license is present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_type: Option<String>,
    /// License ID (if license is present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_id: Option<String>,
    /// Issue date (if license is present)
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub issued_at: Option<time::OffsetDateTime>,
    /// Expiration date (if license is present and has expiration)
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub expires_at: Option<time::OffsetDateTime>,
    /// License version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Verification timestamp
    #[serde(with = "time::serde::rfc3339")]
    pub verified_at: time::OffsetDateTime,
    /// Error message (only present if state is "error" or "invalid")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl From<r_data_core_services::license::service::LicenseVerificationResult> for LicenseStatusDto {
    fn from(result: r_data_core_services::license::service::LicenseVerificationResult) -> Self {
        let state = match result.state {
            r_data_core_services::license::service::LicenseState::None => LicenseStateDto::None,
            r_data_core_services::license::service::LicenseState::Invalid => {
                LicenseStateDto::Invalid
            }
            r_data_core_services::license::service::LicenseState::Error => LicenseStateDto::Error,
            r_data_core_services::license::service::LicenseState::Valid => LicenseStateDto::Valid,
        };

        Self {
            state,
            company: result.company,
            license_type: result.license_type,
            license_id: result.license_id,
            issued_at: result.issued_at,
            expires_at: result.expires_at,
            version: result.version,
            verified_at: result.verified_at,
            error_message: result.error_message,
        }
    }
}

/// Request body for license verification (internal API)
#[derive(Debug, Deserialize)]
pub struct LicenseVerificationRequest {
    /// License key to verify
    pub license_key: String,
}

/// Response for license verification (internal API)
#[derive(Debug, Serialize)]
pub struct LicenseVerificationResponse {
    /// Whether the license is valid
    pub valid: bool,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Component version information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComponentVersionDto {
    /// Name of the component
    pub name: String,
    /// Version string
    pub version: String,
    /// Last time this component was seen (ISO 8601)
    #[serde(with = "time::serde::rfc3339")]
    pub last_seen_at: time::OffsetDateTime,
}

/// System versions response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemVersionsDto {
    /// Core/API server version
    pub core: String,
    /// Worker component version (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker: Option<ComponentVersionDto>,
    /// Maintenance component version (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<ComponentVersionDto>,
}
