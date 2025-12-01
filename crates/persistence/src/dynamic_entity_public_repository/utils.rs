#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use sqlx::PgPool;

/// Get the count of dynamic entity instances for a given entity type
///
/// The entity type corresponds to an entity definition name.
///
/// # Errors
/// Returns an error if the database query fails
pub async fn get_entity_count(pool: &PgPool, entity_type: &str) -> Result<i64> {
    let table_name = format!("{}_entities", entity_type.to_lowercase());

    // Check if table exists first
    let table_exists: bool = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_schema = 'public'
            AND table_name = $1
        ) as "exists!"
        "#,
        table_name
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        return Ok(0);
    }

    let query = format!("SELECT COUNT(*) FROM \"{table_name}\"");
    let count: i64 = sqlx::query_scalar(&query).fetch_one(pool).await?;

    Ok(count)
}
