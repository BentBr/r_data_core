use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Health check response data
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthData {
    /// Current date and time
    pub date: String,

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
    /// Number of items to skip (for pagination)
    pub offset: Option<i64>,
    /// Page number (1-based, alternative to offset)
    pub page: Option<i64>,
    /// Number of items per page (alternative to limit)
    pub per_page: Option<i64>,
}

impl PaginationQuery {
    /// Get the current page number
    pub fn get_page(&self, default: i64) -> i64 {
        self.page.unwrap_or(default)
    }

    /// Get the items per page
    pub fn get_per_page(&self, default: i64, max: i64) -> i64 {
        self.per_page.unwrap_or(default).min(max)
    }

    pub fn get_offset(&self, default: i64) -> i64 {
        self.offset.unwrap_or(default)
    }
}
