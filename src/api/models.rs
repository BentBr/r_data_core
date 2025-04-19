use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Health check response data
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthData {
    /// Current date and time
    pub date: OffsetDateTime,

    /// Generated UUID for this health check
    pub uuid: Uuid,

    /// Route that was accessed
    pub route: String,

    /// User agent that made the request
    pub agent: String,
}

/// Query parameters for paginated API endpoints
/// Used to control the number of items returned and pagination offset
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationQuery {
    /// Maximum number of items to return (defaults to system default if not specified)
    pub limit: Option<i64>,
    /// Number of items to skip (for pagination)
    pub offset: Option<i64>,
}
