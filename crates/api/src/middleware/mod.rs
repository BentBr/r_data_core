mod base_auth;
mod error_handler;
mod error_handlers;

pub use base_auth::AuthMiddlewareService;
pub use error_handler::ErrorHandler;
pub use error_handlers::create_error_handlers;

#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub uuid: uuid::Uuid,
    pub user_uuid: uuid::Uuid,
    pub name: String,
    pub created_at: time::OffsetDateTime,
    pub expires_at: Option<time::OffsetDateTime>,
}
