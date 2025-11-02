pub mod export;
pub mod import;

use sqlx::PgPool;

pub struct AdapterContext {
    pub pool: PgPool,
}
