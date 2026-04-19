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
    pub company: Option<String>,
    /// License type (if license is present)
    pub license_type: Option<String>,
    /// License ID (if license is present)
    pub license_id: Option<String>,
    /// Issue date (if license is present)
    #[serde(with = "time::serde::rfc3339::option")]
    pub issued_at: Option<time::OffsetDateTime>,
    /// Expiration date (if license is present and has expiration)
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<time::OffsetDateTime>,
    /// License version
    pub version: Option<String>,
    /// Verification timestamp
    #[serde(with = "time::serde::rfc3339")]
    pub verified_at: time::OffsetDateTime,
    /// Error message (only present if state is "error" or "invalid")
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
    pub message: Option<String>,
}

/// Response for system capabilities (which optional features are configured)
#[derive(Debug, Serialize, ToSchema)]
pub struct CapabilitiesResponse {
    /// Whether system mail is configured (enables password reset etc.)
    pub system_mail_configured: bool,
    /// Whether workflow mail is configured (enables email outputs in workflows)
    pub workflow_mail_configured: bool,
}

/// Query parameters for filtering system logs
#[derive(Debug, Deserialize, ToSchema)]
pub struct SystemLogQuery {
    /// Page number (1-based, default: 1)
    pub page: Option<i64>,
    /// Items per page (default: 20, max: 100)
    pub page_size: Option<i64>,
    /// Filter by log type
    pub log_type: Option<r_data_core_core::system_log::SystemLogType>,
    /// Filter by resource type
    pub resource_type: Option<r_data_core_core::system_log::SystemLogResourceType>,
    /// Filter by status
    pub status: Option<r_data_core_core::system_log::SystemLogStatus>,
}

impl SystemLogQuery {
    /// Convert to (limit, offset, page, `per_page`) with defaults
    #[must_use]
    pub fn to_pagination(&self) -> (i64, i64, i64, i64) {
        let per_page = self.page_size.unwrap_or(20).clamp(1, 100);
        let page = self.page.unwrap_or(1).max(1);
        let offset = (page - 1) * per_page;
        (per_page, offset, page, per_page)
    }
}

/// Single system log entry response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SystemLogDto {
    /// Log entry UUID
    pub uuid: String,
    /// When this log entry was created
    pub created_at: String,
    /// UUID of the user that triggered the event (if known)
    pub created_by: Option<String>,
    /// Status of the logged event
    pub status: r_data_core_core::system_log::SystemLogStatus,
    /// Type of log entry
    pub log_type: r_data_core_core::system_log::SystemLogType,
    /// Type of resource this log entry relates to
    pub resource_type: r_data_core_core::system_log::SystemLogResourceType,
    /// UUID of the affected resource (if applicable)
    pub resource_uuid: Option<String>,
    /// Short human-readable summary
    pub summary: String,
    /// Optional structured details (JSONB)
    pub details: Option<serde_json::Value>,
}

impl From<r_data_core_core::system_log::SystemLog> for SystemLogDto {
    fn from(log: r_data_core_core::system_log::SystemLog) -> Self {
        use time::format_description::well_known::Rfc3339;
        Self {
            uuid: log.uuid.to_string(),
            created_at: log
                .created_at
                .format(&Rfc3339)
                .unwrap_or_else(|_| log.created_at.to_string()),
            created_by: log.created_by.map(|u| u.to_string()),
            status: log.status,
            log_type: log.log_type,
            resource_type: log.resource_type,
            resource_uuid: log.resource_uuid.map(|u| u.to_string()),
            summary: log.summary,
            details: log.details,
        }
    }
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
    pub worker: Option<ComponentVersionDto>,
    /// Maintenance component version (if available)
    pub maintenance: Option<ComponentVersionDto>,
}
