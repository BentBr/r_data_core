use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;
use crate::error::Result;
use serde_json::Value;

pub struct DynamicEntityRepository {
    db_pool: PgPool,
}

impl DynamicEntityRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn count_entity_instances(&self, entity_type: &str) -> Result<i64> {
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
        .fetch_one(&self.db_pool)
        .await?;

        if !table_exists {
            return Ok(0);
        }

        let query = format!("SELECT COUNT(*) FROM \"{}\"", table_name);
        let count: i64 = sqlx::query_scalar(&query).fetch_one(&self.db_pool).await?;

        Ok(count)
    }
} 