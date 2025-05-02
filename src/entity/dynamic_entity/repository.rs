use log::{debug, error, warn};
use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, Column, FromRow, PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::entity::class::definition::ClassDefinition;
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::dynamic_entity::mapper;
use crate::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
use crate::entity::dynamic_entity::utils;
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
        // Get the class definition to validate against
        let class_def = utils::get_class_definition(&self.pool, &entity.entity_type).await?;

        // Validate the entity against the class definition
        entity.validate()?;

        // Extract UUID from the entity
        let uuid = entity
            .field_data
            .get("uuid")
            .and_then(|v| match v {
                JsonValue::String(s) => Uuid::parse_str(s).ok(),
                _ => None,
            })
            .ok_or_else(|| Error::Validation("Entity is missing a valid UUID".to_string()))?;

        // Extract the path or generate a default one
        let path = entity
            .field_data
            .get("path")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("/{}/{}", entity.entity_type.to_lowercase(), uuid));

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // First, insert into entities_registry
        let registry_query = "
            INSERT INTO entities_registry
                (uuid, entity_type, path, created_at, updated_at, created_by, updated_by, published, version)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ";

        // Extract metadata fields or use defaults
        let created_at = entity
            .field_data
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(|s| {
                OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| OffsetDateTime::now_utc())
            })
            .unwrap_or_else(OffsetDateTime::now_utc);

        let updated_at = entity
            .field_data
            .get("updated_at")
            .and_then(|v| v.as_str())
            .map(|s| {
                OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| OffsetDateTime::now_utc())
            })
            .unwrap_or_else(OffsetDateTime::now_utc);

        let created_by = entity
            .field_data
            .get("created_by")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());

        let updated_by = entity
            .field_data
            .get("updated_by")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());

        let published = entity
            .field_data
            .get("published")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let version = entity
            .field_data
            .get("version")
            .and_then(|v| v.as_i64())
            .unwrap_or(1);

        sqlx::query(registry_query)
            .bind(uuid)
            .bind(&entity.entity_type)
            .bind(path)
            .bind(created_at)
            .bind(updated_at)
            .bind(created_by)
            .bind(updated_by)
            .bind(published)
            .bind(version)
            .execute(&mut *tx)
            .await?;

        // Then, insert custom fields into the entity-specific table
        let table_name = utils::get_table_name(&entity.entity_type);

        // Get column names for this table
        let columns_result = sqlx::query(
            "SELECT column_name
             FROM information_schema.columns
             WHERE table_schema = 'public' AND table_name = $1",
        )
        .bind(&table_name)
        .fetch_all(&mut *tx)
        .await?;

        // Extract column names
        let valid_columns: Vec<String> = columns_result
            .iter()
            .map(|row| {
                row.try_get::<String, _>("column_name")
                    .unwrap_or_default()
                    .to_lowercase()
            })
            .collect();

        // Registry fields that should not be included in the entity table
        let registry_fields = [
            "entity_type",
            "path",
            "created_at",
            "updated_at",
            "created_by",
            "updated_by",
            "published",
            "version",
        ];

        // Build query for entity-specific table
        let mut columns = vec!["uuid".to_string()];
        let mut values = vec![format!("'{}'", uuid)];

        // Add entity-specific fields
        for (key, value) in &entity.field_data {
            if registry_fields.contains(&key.as_str()) || key == "uuid" {
                continue; // Skip fields that are stored in entities_registry
            }

            if valid_columns.contains(&key.to_lowercase()) {
                columns.push(key.clone());

                // Format the value appropriately based on its type
                let value_str = match value {
                    JsonValue::String(s) => format!("'{}'", s.replace("'", "''")),
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::Bool(b) => {
                        if *b {
                            "TRUE".to_string()
                        } else {
                            "FALSE".to_string()
                        }
                    }
                    JsonValue::Null => "NULL".to_string(),
                    _ => format!("'{}'", value.to_string().replace("'", "''")), // For complex types
                };

                values.push(value_str);
            }
        }

        // Create the INSERT statement for the entity table
        if columns.len() > 1 {
            // If we have more than just the UUID
            let query = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                columns.join(", "),
                values.join(", ")
            );

            sqlx::query(&query).execute(&mut *tx).await?;
        } else {
            // If we only have the UUID, just insert that
            sqlx::query(&format!("INSERT INTO {} (uuid) VALUES ($1)", table_name))
                .bind(uuid)
                .execute(&mut *tx)
                .await?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    /// Update an existing dynamic entity
    pub async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        // Validate the entity against the class definition
        entity.validate()?;

        // Extract UUID from the entity
        let uuid = entity
            .field_data
            .get("uuid")
            .and_then(|v| match v {
                JsonValue::String(s) => Uuid::parse_str(s).ok(),
                _ => None,
            })
            .ok_or_else(|| Error::Validation("Entity is missing a valid UUID".to_string()))?;

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // 1. Update entities_registry table
        let mut registry_fields = Vec::new();
        let mut registry_values = Vec::new();

        // Extract metadata fields for update
        if let Some(path) = entity.field_data.get("path").and_then(|v| v.as_str()) {
            registry_fields.push("path = $1");
            registry_values.push(path.to_string());
        }

        if let Some(published) = entity.field_data.get("published").and_then(|v| v.as_bool()) {
            registry_fields.push("published = $2");
            registry_values.push(published.to_string());
        }

        let updated_by = entity
            .field_data
            .get("updated_by")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());

        if updated_by.is_some() {
            registry_fields.push("updated_by = $3");
            registry_values.push(updated_by.unwrap().to_string());
        }

        // Always update timestamp and increment version
        let update_registry_query = if registry_fields.is_empty() {
            // Just update the timestamp and version
            String::from(
                "UPDATE entities_registry SET updated_at = NOW(), version = version + 1 
                WHERE uuid = $1 AND entity_type = $2",
            )
        } else {
            format!(
                "UPDATE entities_registry SET {}, updated_at = NOW(), version = version + 1 
                    WHERE uuid = $4 AND entity_type = $5",
                registry_fields.join(", ")
            )
        };

        // Create a query builder
        let mut registry_query = sqlx::query(&update_registry_query);

        // Bind values for the set clauses
        for value in &registry_values {
            registry_query = registry_query.bind(value);
        }

        // Always bind UUID and entity_type
        registry_query = registry_query.bind(uuid).bind(&entity.entity_type);

        // Execute the registry update
        registry_query.execute(&mut *tx).await?;

        // 2. Update entity-specific table
        let table_name = utils::get_table_name(&entity.entity_type);

        // Get column names for this table
        let columns_result = sqlx::query(
            "SELECT column_name
             FROM information_schema.columns
             WHERE table_schema = 'public' AND table_name = $1",
        )
        .bind(&table_name)
        .fetch_all(&mut *tx)
        .await?;

        // Extract column names
        let valid_columns: Vec<String> = columns_result
            .iter()
            .map(|row| {
                row.try_get::<String, _>("column_name")
                    .unwrap_or_default()
                    .to_lowercase()
            })
            .collect();

        // Registry fields that should not be included in the entity table
        let registry_fields = [
            "entity_type",
            "path",
            "created_at",
            "updated_at",
            "created_by",
            "updated_by",
            "published",
            "version",
        ];

        // Build SET clauses for entity-specific fields
        let mut set_clauses = Vec::new();

        for (key, value) in &entity.field_data {
            if registry_fields.contains(&key.as_str()) || key == "uuid" {
                continue; // Skip fields that are stored in entities_registry
            }

            if valid_columns.contains(&key.to_lowercase()) {
                // Format the value appropriately based on its type
                let value_str = match value {
                    JsonValue::String(s) => format!("'{}'", s.replace("'", "''")),
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::Bool(b) => {
                        if *b {
                            "TRUE".to_string()
                        } else {
                            "FALSE".to_string()
                        }
                    }
                    JsonValue::Null => "NULL".to_string(),
                    _ => format!("'{}'", value.to_string().replace("'", "''")), // For complex types
                };

                set_clauses.push(format!("{} = {}", key, value_str));
            }
        }

        // Execute the entity update if we have SET clauses
        if !set_clauses.is_empty() {
            let update_entity_query = format!(
                "UPDATE {} SET {} WHERE uuid = '{}'",
                table_name,
                set_clauses.join(", "),
                uuid
            );

            sqlx::query(&update_entity_query).execute(&mut *tx).await?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    /// Filter entities based on field values
    pub async fn filter_entities(
        &self,
        entity_type: &str,
        filters: &HashMap<String, JsonValue>,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        debug!(
            "Filtering entities of type {} with filters: {:?}",
            entity_type, filters
        );

        // Get the class definition to understand entity structure
        let class_def = utils::get_class_definition(&self.pool, entity_type).await?;

        // Get the view name
        let view_name = utils::get_view_name(entity_type);

        // Build WHERE clause from filters
        let (where_clause, params) = utils::build_where_clause(filters, &class_def);

        // Build the query with field selection
        let query = if let Some(fields) = &exclusive_fields {
            // Always include system fields
            let mut selected_fields = vec![
                "uuid".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
                "created_by".to_string(),
                "updated_by".to_string(),
                "published".to_string(),
                "version".to_string(),
                "path".to_string(),
            ];

            // Add requested fields
            for field in fields {
                if !selected_fields.contains(field) {
                    selected_fields.push(field.clone());
                }
            }

            format!(
                "SELECT {} FROM {} WHERE {}",
                selected_fields.join(", "),
                view_name,
                where_clause
            )
        } else {
            format!("SELECT * FROM {} WHERE {}", view_name, where_clause)
        };

        // Add pagination
        let mut final_query = query + &format!(" LIMIT {} OFFSET {}", limit, offset);

        debug!("Query: {}", final_query);

        // Execute the query
        let mut query_builder = sqlx::query(&final_query);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        // Convert rows to DynamicEntity
        let mut entities = Vec::new();

        debug!("Rows: {:?}", rows);

        for row in rows {
            debug!("Row: {:?}", row);
            let field_data = mapper::extract_field_data(&row);
            debug!("field data: {:?}", field_data);
            let entity =
                mapper::create_entity(entity_type.to_string(), field_data, class_def.clone());
            entities.push(entity);
        }

        Ok(entities)
    }

    /// Count entities of a specific type
    pub async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        // Use the view for this entity type
        let view_name = utils::get_view_name(entity_type);

        // Check if view exists
        let view_exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM information_schema.tables
                WHERE table_schema = 'public'
                AND table_name = $1
            ) AS "exists!"
            "#,
            &view_name
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        if !view_exists {
            return Err(Error::NotFound(format!(
                "Entity type '{}' not found",
                entity_type
            )));
        }

        // Query count
        let query = format!("SELECT COUNT(*) FROM {}", view_name);
        let count: i64 = sqlx::query_scalar(&query)
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(count)
    }
}

#[async_trait::async_trait]
impl DynamicEntityRepositoryTrait for DynamicEntityRepository {
    async fn create(&self, entity: &DynamicEntity) -> Result<()> {
        self.create(entity).await
    }

    async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        self.update(entity).await
    }

    async fn get_by_type(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Option<DynamicEntity>> {
        debug!("Getting entity of type {} with UUID {}", entity_type, uuid);

        // Get the class definition to understand entity structure
        let class_def = utils::get_class_definition(&self.pool, entity_type).await?;

        // Get the view name
        let view_name = utils::get_view_name(entity_type);

        // Build the query with field selection
        let query = if let Some(fields) = &exclusive_fields {
            // Always include system fields
            let mut selected_fields = vec![
                "uuid".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
                "created_by".to_string(),
                "updated_by".to_string(),
                "published".to_string(),
                "version".to_string(),
                "path".to_string(),
            ];

            // Add requested fields
            for field in fields {
                if !selected_fields.contains(field) {
                    selected_fields.push(field.clone());
                }
            }

            format!(
                "SELECT {} FROM {} WHERE uuid = $1",
                selected_fields.join(", "),
                view_name
            )
        } else {
            format!("SELECT * FROM {} WHERE uuid = $1", view_name)
        };

        debug!("Query: {}", query);

        let row = sqlx::query(&query)
            .bind(uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Error fetching entity: {:?}", e);
                Error::Database(e)
            })?;

        if let Some(row) = row {
            // Map the row to a DynamicEntity
            let entity = mapper::map_row_to_entity(&row, entity_type, &class_def);
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }

    async fn get_all_by_type(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        debug!("Getting all entities of type {}", entity_type);

        // Get the class definition to understand entity structure
        let class_def = utils::get_class_definition(&self.pool, entity_type).await?;

        // Get the view name
        let view_name = utils::get_view_name(entity_type);

        // Build the query with field selection
        let query = if let Some(fields) = &exclusive_fields {
            // Always include system fields
            let mut selected_fields = vec![
                "uuid".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
                "created_by".to_string(),
                "updated_by".to_string(),
                "published".to_string(),
                "version".to_string(),
                "path".to_string(),
            ];

            // Add requested fields
            for field in fields {
                if !selected_fields.contains(field) {
                    selected_fields.push(field.clone());
                }
            }

            format!(
                "SELECT {} FROM {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                selected_fields.join(", "),
                view_name
            )
        } else {
            format!(
                "SELECT * FROM {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                view_name
            )
        };

        debug!("Query: {}", query);

        // Query all entities
        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!("Error fetching entities: {:?}", e);
                Error::Database(e)
            })?;

        // Convert rows to DynamicEntity objects
        let entities = rows
            .iter()
            .map(|row| mapper::map_row_to_entity(row, entity_type, &class_def))
            .collect();

        Ok(entities)
    }

    async fn delete_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<()> {
        debug!("Deleting entity of type {} with UUID {}", entity_type, uuid);

        // Get the table name
        let table_name = utils::get_table_name(entity_type);

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // First, delete from the entity-specific table
        let query = format!("DELETE FROM {} WHERE uuid = $1", table_name);

        let result = sqlx::query(&query).bind(uuid).execute(&mut *tx).await;

        // If the entity table doesn't exist, just log a warning
        if let Err(e) = result {
            warn!("Error deleting from {}: {}", table_name, e);
        }

        // Then delete from entities_registry
        sqlx::query("DELETE FROM entities_registry WHERE uuid = $1 AND entity_type = $2")
            .bind(uuid)
            .bind(entity_type)
            .execute(&mut *tx)
            .await
            .map_err(Error::Database)?;

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    async fn filter_entities(
        &self,
        entity_type: &str,
        filters: &HashMap<String, JsonValue>,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        self.filter_entities(entity_type, filters, limit, offset, exclusive_fields)
            .await
    }

    async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        self.count_entities(entity_type).await
    }
}
