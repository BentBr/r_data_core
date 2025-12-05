#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use time::OffsetDateTime;
use uuid::Uuid;

/// API key info attached to the request
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub uuid: Uuid,
    pub user_uuid: Uuid,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
}
