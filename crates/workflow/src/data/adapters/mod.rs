pub mod auth;
pub mod destination;
pub mod format;
pub mod source;

use sqlx::PgPool;

pub struct AdapterContext {
    pub pool: PgPool,
}
