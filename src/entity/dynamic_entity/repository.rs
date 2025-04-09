use serde_json::Value;
use sqlx::{query, types::Json, PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::entity::DynamicEntity;
use crate::entity::class::ClassDefinition;
use crate::error::{Error, Result};

/// Repository for CRUD operations on dynamic entities
pub struct DynamicEntityRepository {
    /// Database connection pool
    pub pool: PgPool,
    /// Entity type
    pub entity_type: String,
    /// Table name
    pub table_name: String,
    /// Class definition
    pub definition: Option<Arc<ClassDefinition>>,
}

impl DynamicEntityRepository {
    /// Create a new repository for a dynamic entity type
    pub fn new(
        pool: PgPool,
        entity_type: String,
        definition: Option<Arc<ClassDefinition>>,
    ) -> Self {
        let table_name = format!("entity_{}", entity_type.to_lowercase());
        Self {
            pool,
            entity_type,
            table_name,
            definition,
        }
    }

    /// Create a new entity
    pub async fn create(&self, entity: &DynamicEntity) -> Result<Uuid> {
        if let Some(def) = &self.definition {
            entity.validate(def)?;
        }

        let uuid = entity.get::<Uuid>("uuid")?;
        query(&format!(
            "INSERT INTO {} (uuid, data) VALUES ($1, $2)",
            self.table_name
        ))
        .bind(uuid)
        .bind(Json(&entity.data))
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(uuid)
    }

    /// Get an entity by UUID
    pub async fn get(&self, uuid: Uuid) -> Result<DynamicEntity> {
        let row = query(&format!(
            "SELECT data FROM {} WHERE uuid = $1",
            self.table_name
        ))
        .bind(uuid)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                Error::NotFound(format!("Entity with UUID {} not found", uuid))
            }
            _ => Error::Database(e),
        })?;

        let data: Json<HashMap<String, Value>> =
            row.try_get("data").map_err(Error::Database)?;

        Ok(DynamicEntity::from_data(
            self.entity_type.clone(),
            data.0,
            self.definition.clone(),
        ))
    }

    /// Update an entity
    pub async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        if let Some(def) = &self.definition {
            entity.validate(def)?;
        }

        let uuid = entity.get::<Uuid>("uuid")?;
        let result = query(&format!(
            "UPDATE {} SET data = $1 WHERE uuid = $2",
            self.table_name
        ))
        .bind(Json(&entity.data))
        .bind(uuid)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Entity with UUID {} not found", uuid)));
        }

        Ok(())
    }

    /// Delete an entity
    pub async fn delete(&self, uuid: Uuid) -> Result<()> {
        let result = query(&format!("DELETE FROM {} WHERE uuid = $1", self.table_name))
            .bind(uuid)
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Entity with UUID {} not found",
                uuid
            )));
        }

        Ok(())
    }

    /// List entities with optional filters, limit and offset
    pub async fn list(
        &self,
        filters: Option<HashMap<String, Value>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<DynamicEntity>> {
        // Build the query
        let mut query_str = format!("SELECT data FROM {}", self.table_name);

        // Add WHERE clause for filters
        if let Some(filters) = &filters {
            if !filters.is_empty() {
                query_str.push_str(" WHERE ");

                let mut conditions = Vec::new();
                for (key, value) in filters {
                    conditions.push(format!(
                        "data->>'{}' = '{}'",
                        key,
                        value.to_string().replace("'", "''")
                    ));
                }

                query_str.push_str(&conditions.join(" AND "));
            }
        }

        // Add limit and offset
        if let Some(limit) = limit {
            query_str.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query_str.push_str(&format!(" OFFSET {}", offset));
        }

        // Execute the query
        let rows = sqlx::query(&query_str)
            .fetch_all(&self.pool)
            .await
            .map_err(Error::Database)?;

        // Convert rows to entities
        let mut entities = Vec::with_capacity(rows.len());
        for row in rows {
            let data: Json<HashMap<String, Value>> =
                row.try_get("data").map_err(Error::Database)?;

            entities.push(DynamicEntity::from_data(
                self.entity_type.clone(),
                data.0,
                self.definition.clone(),
            ));
        }

        Ok(entities)
    }
}
