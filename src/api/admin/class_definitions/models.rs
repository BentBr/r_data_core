use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// Pagination query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Path parameter for UUID
#[derive(Debug, Deserialize, ToSchema)]
pub struct PathUuid {
    pub uuid: Uuid,
}
