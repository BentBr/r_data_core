use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

/// Health check response data
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct HealthData {
    /// Current date and time
    pub date: String,

    /// Generated UUID for this health check
    #[ts(type = "string")]
    pub uuid: Uuid,

    /// Route that was accessed
    pub route: String,

    /// User agent that made the request
    pub agent: String,
}
