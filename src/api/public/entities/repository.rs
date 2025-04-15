use super::models::EntityTypeInfo;
use crate::entity::class::definition::ClassDefinition;
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::field::FieldDefinition;
use crate::error::Result;
use serde_json::Value;
use sqlx::Column;
use sqlx::{postgres::PgRow, PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

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

    pub async fn get_entity(&self, entity_type: &str, uuid: &Uuid) -> Result<DynamicEntity> {
        // Get class definition to understand entity structure
        let class_def_result = sqlx::query!(
            r#"
            SELECT entity_type, display_name, description, 
                   group_name, allow_children, icon 
            FROM class_definitions
            WHERE entity_type = $1 AND published = true
            "#,
            entity_type
        )
        .fetch_optional(&self.db_pool)
        .await?;

        let class_def = if let Some(row) = class_def_result {
            // Convert row to ClassDefinition
            ClassDefinition::new(
                row.entity_type,
                row.display_name,
                row.description,
                row.group_name,
                row.allow_children,
                row.icon,
                Vec::<FieldDefinition>::new(), // Empty fields for now
            )
        } else {
            return Err(crate::error::Error::NotFound(format!(
                "Class definition for entity type {} not found",
                entity_type
            )));
        };

        // Build table name
        let table_name = class_def.get_table_name();

        // Query the entity
        let query = format!("SELECT * FROM {} WHERE uuid = $1", table_name);

        let row = sqlx::query(&query)
            .bind(uuid)
            .fetch_optional(&self.db_pool)
            .await?;

        if let Some(row) = row {
            // Convert row to DynamicEntity
            let mut field_data = HashMap::new();
            for column in row.columns().iter() {
                let column_name = column.name();
                if let Ok(value) = row.try_get::<Value, _>(column_name) {
                    field_data.insert(column_name.to_string(), value);
                }
            }

            // Create DynamicEntity
            let entity =
                DynamicEntity::from_data(entity_type.to_string(), field_data, Arc::new(class_def));

            Ok(entity)
        } else {
            Err(crate::error::Error::NotFound(format!(
                "Entity with UUID {} not found",
                uuid
            )))
        }
    }

    // Additional repository methods would be implemented here
    // ...
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
