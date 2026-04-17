#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

/// Request to create a new API key
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct CreateApiKeyRequest {
    /// Name of the API key
    pub name: String,
    /// Optional description for the API key
    pub description: Option<String>,
    /// Number of days until expiration (default: 365)
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expires_in_days: Option<i64>,
}

/// Response containing API key information
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct ApiKeyResponse {
    /// UUID of the API key
    #[ts(type = "string")]
    pub uuid: Uuid,
    /// Name of the API key
    pub name: String,
    /// Description of the API key
    pub description: Option<String>,
    /// Whether the API key is active
    pub is_active: bool,
    /// When the API key was created
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    /// When the API key expires (if applicable)
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(type = "string | null")]
    pub expires_at: Option<OffsetDateTime>,
    /// When the API key was last used
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(type = "string | null")]
    pub last_used_at: Option<OffsetDateTime>,
    /// UUID of the user who created this key
    #[ts(type = "string")]
    pub created_by: Uuid,
    /// UUID of the user to whom this key is assigned
    #[ts(type = "string")]
    pub user_uuid: Uuid,
    /// Whether the key is published
    pub published: bool,
}

/// Response when an API key is created (includes the actual key value)
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct ApiKeyCreatedResponse {
    /// UUID of the API key
    #[ts(type = "string")]
    pub uuid: Uuid,
    /// Name of the API key
    pub name: String,
    /// The actual API key value (only shown once at creation)
    pub api_key: String,
    /// Description of the API key
    pub description: Option<String>,
    /// Whether the API key is active
    pub is_active: bool,
    /// When the API key was created
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    /// When the API key expires (if applicable)
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(type = "string | null")]
    pub expires_at: Option<OffsetDateTime>,
    /// UUID of the user who created this key
    #[ts(type = "string")]
    pub created_by: Uuid,
    /// UUID of the user to whom this key is assigned
    #[ts(type = "string")]
    pub user_uuid: Uuid,
    /// Whether the key is published
    pub published: bool,
    /// When the API key was last used
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(type = "string | null")]
    pub last_used_at: Option<OffsetDateTime>,
}

/// Request to reassign an API key to a different user
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct ReassignApiKeyRequest {
    /// UUID of the user to reassign the API key to
    #[ts(type = "string")]
    pub user_uuid: Uuid,
}
