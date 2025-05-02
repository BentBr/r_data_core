use super::models::EntityTypeInfo;
use crate::error::Result;
use sqlx::PgPool;

pub struct EntityRepository {
    db_pool: PgPool,
}

impl EntityRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn list_available_entities(&self) -> Result<Vec<EntityTypeInfo>> {
        let rows = sqlx::query!(
            r#"
            SELECT entity_type as name, display_name, description, 
                   uuid as class_definition_uuid
            FROM class_definitions
            WHERE published = true
            "#,
        )
        .fetch_all(&self.db_pool)
        .await?;

        let mut result = Vec::new();

        for row in rows {
            // Count entity instances
            let entity_count = get_entity_count(&self.db_pool, &row.name)
                .await
                .unwrap_or(0);

            // Count fields for each entity
            let field_count: i64 = match sqlx::query_scalar!(
                r#"
                SELECT COUNT(*) as count 
                FROM class_definitions 
                WHERE entity_type = $1
                "#,
                row.name
            )
            .fetch_one(&self.db_pool)
            .await
            {
                Ok(count) => count.unwrap_or(0),
                Err(_) => 0,
            };

            result.push(EntityTypeInfo {
                name: row.name,
                display_name: row.display_name,
                description: row.description,
                is_system: false,
                entity_count,
                field_count: field_count as i32,
            });
        }

        Ok(result)
    }
}

async fn get_entity_count(pool: &PgPool, entity_type: &str) -> Result<i64> {
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

    let query = format!("SELECT COUNT(*) FROM \"{}\"", table_name);
    let count: i64 = sqlx::query_scalar(&query).fetch_one(pool).await?;

    Ok(count)
}
