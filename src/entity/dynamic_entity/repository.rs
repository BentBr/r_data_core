use chrono::Utc;
use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, FromRow, PgPool};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::entity::class::definition::ClassDefinition;
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::DynamicFields;
use crate::error::{Error, Result};

/// Repository for managing dynamic entities
pub struct DynamicEntityRepository {
    /// Database connection pool
    pub pool: PgPool,
}

impl DynamicEntityRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new dynamic entity
    pub async fn create(&self, entity: &DynamicEntity) -> Result<()> {
        // First get the class definition to validate against
        let class_def = sqlx::query("SELECT * FROM class_definitions WHERE entity_type = $1")
            .bind(&entity.entity_type)
            .map(|row: PgRow| {
                // Use the FromRow trait to convert the row
                ClassDefinition::from_row(&row).expect("Error converting row to ClassDefinition")
            })
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Class definition for type {} not found",
                    entity.entity_type
                ))
            })?;

        // Validate the entity against the class definition
        entity.validate()?;

        // Generate UUID
        let context = uuid::ContextV7::new();
        let ts = uuid::timestamp::Timestamp::now(&context);
        let uuid = Uuid::new_v7(ts);

        // Get path from entity or generate default
        let path = entity
            .get_field("path")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("/{}", entity.entity_type.to_lowercase()));

        // Insert the entity into the database
        sqlx::query!(
            r#"
            INSERT INTO entities (
                uuid, path, entity_type, created_at, updated_at, created_by, updated_by, 
                published, version, field_data
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            uuid,
            path,
            entity.entity_type,
            Utc::now(),
            Utc::now(),
            None::<Uuid>, // created_by
            None::<Uuid>, // updated_by
            false,        // published
            1,            // version
            JsonValue::Object(entity.field_data.clone().into_iter().collect())
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update an existing dynamic entity
    pub async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        // Get the class definition to validate against
        let class_def = sqlx::query("SELECT * FROM class_definitions WHERE entity_type = $1")
            .bind(&entity.entity_type)
            .map(|row: PgRow| {
                // Use the FromRow trait to convert the row
                ClassDefinition::from_row(&row).expect("Error converting row to ClassDefinition")
            })
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Class definition for type {} not found",
                    entity.entity_type
                ))
            })?;

        // Validate the entity against the class definition
        entity.validate()?;

        // Update the entity in the database
        sqlx::query!(
            r#"
            UPDATE entities
            SET field_data = $1, updated_at = $2, version = version + 1
            WHERE entity_type = $3
            "#,
            JsonValue::Object(entity.field_data.clone().into_iter().collect()),
            Utc::now(),
            entity.entity_type
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a dynamic entity by its type
    pub async fn get_by_type(&self, entity_type: &str) -> Result<Option<DynamicEntity>> {
        // Get the class definition
        let class_def = sqlx::query("SELECT * FROM class_definitions WHERE entity_type = $1")
            .bind(entity_type)
            .map(|row: PgRow| {
                // Use the FromRow trait to convert the row
                ClassDefinition::from_row(&row).expect("Error converting row to ClassDefinition")
            })
            .fetch_optional(&self.pool)
            .await?;

        if let Some(class_def) = class_def {
            // Get the entity data
            let row = sqlx::query!(
                r#"
                SELECT * FROM entities WHERE entity_type = $1
                "#,
                entity_type
            )
            .fetch_optional(&self.pool)
            .await?;

            if let Some(row) = row {
                let field_data: HashMap<String, JsonValue> =
                    serde_json::from_value(row.field_data)?;
                Ok(Some(DynamicEntity {
                    entity_type: row.entity_type,
                    field_data,
                    definition: Arc::new(class_def),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Delete a dynamic entity by its type
    pub async fn delete_by_type(&self, entity_type: &str) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM entities WHERE entity_type = $1
            "#,
            entity_type
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
