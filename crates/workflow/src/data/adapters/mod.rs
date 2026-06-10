pub mod auth;
pub mod destination;
pub mod format;
pub(crate) mod http;
pub mod source;

use sqlx::PgPool;

pub struct AdapterContext {
    pub pool: PgPool,
}
