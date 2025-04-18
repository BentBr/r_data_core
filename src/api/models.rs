use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Health check response data
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthData {
    /// Current date and time
    pub date: DateTime<Utc>,

    /// Generated UUID for this health check
    pub uuid: Uuid,

    /// Route that was accessed
    pub route: String,

    /// User agent that made the request
    pub agent: String,
}
