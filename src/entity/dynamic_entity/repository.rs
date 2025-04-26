use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, FromRow, PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use time;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::entity::class::definition::ClassDefinition;
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
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
        let table_name = format!("entity_{}", entity.entity_type.to_lowercase());

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
        let table_name = format!("entity_{}", entity.entity_type.to_lowercase());

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
                SELECT * FROM entities_registry WHERE entity_type = $1
                "#,
                entity_type
            )
            .fetch_optional(&self.pool)
            .await?;

            if let Some(row) = row {
                let field_data: HashMap<String, JsonValue> = {
                    // Create a HashMap containing all fields from the row
                    let mut data = HashMap::new();
                    data.insert("uuid".to_string(), JsonValue::String(row.uuid.to_string()));
                    data.insert(
                        "entity_type".to_string(),
                        JsonValue::String(row.entity_type.clone()),
                    );
                    data.insert("path".to_string(), JsonValue::String(row.path.clone()));

                    // Add dates properly formatted
                    data.insert(
                        "created_at".to_string(),
                        JsonValue::String(row.created_at.to_string()),
                    );
                    data.insert(
                        "updated_at".to_string(),
                        JsonValue::String(row.updated_at.to_string()),
                    );

                    // Add optional fields checking for nulls
                    data.insert(
                        "created_by".to_string(),
                        JsonValue::String(row.created_by.to_string()),
                    );
                    if let Some(updated_by) = row.updated_by {
                        data.insert(
                            "updated_by".to_string(),
                            JsonValue::String(updated_by.to_string()),
                        );
                    }

                    data.insert("published".to_string(), JsonValue::Bool(row.published));
                    data.insert(
                        "version".to_string(),
                        JsonValue::Number(serde_json::Number::from(row.version)),
                    );

                    data
                };
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
            DELETE FROM entities_registry WHERE entity_type = $1
            "#,
            entity_type
        )
        .execute(&self.pool)
        .await?;

        Ok(())
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

    async fn get_by_type(&self, entity_type: &str) -> Result<Option<DynamicEntity>> {
        self.get_by_type(entity_type).await
    }

    async fn get_all_by_type(&self, entity_type: &str) -> Result<Vec<DynamicEntity>> {
        let table_name = format!("entity_{}", entity_type.to_lowercase());

        // Get the class definition
        let class_def = sqlx::query("SELECT * FROM class_definitions WHERE entity_type = $1")
            .bind(entity_type)
            .map(|row: PgRow| {
                ClassDefinition::from_row(&row).expect("Error converting row to ClassDefinition")
            })
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Class definition for type {} not found",
                    entity_type
                ))
            })?;

        // Create a shared Arc for the class definition
        let class_def_arc = Arc::new(class_def);

        // Query the registry and entity table
        let query = format!(
            "SELECT r.*, e.* 
             FROM entities_registry r
             JOIN {} e ON r.uuid = e.uuid
             WHERE r.entity_type = $1",
            table_name
        );

        let rows = sqlx::query(&query)
            .bind(entity_type)
            .fetch_all(&self.pool)
            .await?;

        // Convert rows to DynamicEntity objects
        let mut entities = Vec::new();
        for row in rows {
            let mut field_data = HashMap::new();

            // Add system fields from registry
            field_data.insert(
                "uuid".to_string(),
                JsonValue::String(row.get::<Uuid, _>("uuid").to_string()),
            );
            field_data.insert(
                "entity_type".to_string(),
                JsonValue::String(row.get::<String, _>("entity_type")),
            );
            field_data.insert(
                "path".to_string(),
                JsonValue::String(row.get::<String, _>("path")),
            );
            // Add other system fields

            // Add custom fields from the entity table
            for field in &class_def_arc.fields {
                let field_name = &field.name;
                if let Ok(value) = row.try_get::<JsonValue, _>(field_name.as_str()) {
                    field_data.insert(field_name.clone(), value);
                }
            }

            entities.push(DynamicEntity {
                entity_type: entity_type.to_string(),
                field_data,
                definition: class_def_arc.clone(),
            });
        }

        Ok(entities)
    }

    async fn delete_by_type(&self, entity_type: &str) -> Result<()> {
        self.delete_by_type(entity_type).await
    }

    async fn filter_entities(
        &self,
        entity_type: &str,
        filters: &HashMap<String, JsonValue>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DynamicEntity>> {
        let table_name = format!("entity_{}", entity_type.to_lowercase());

        // Get the class definition
        let class_def = sqlx::query("SELECT * FROM class_definitions WHERE entity_type = $1")
            .bind(entity_type)
            .map(|row: PgRow| {
                ClassDefinition::from_row(&row).expect("Error converting row to ClassDefinition")
            })
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "Class definition for type {} not found",
                    entity_type
                ))
            })?;

        // Create a shared Arc for the class definition
        let class_def_arc = Arc::new(class_def);

        // Build the WHERE clause for filtering
        let mut where_clauses = Vec::new();
        let mut bind_params: Vec<String> = Vec::new();

        for (field, value) in filters {
            // Handle registry fields vs entity fields
            let registry_fields = ["uuid", "entity_type", "path", "created_at", "updated_at"];

            if registry_fields.contains(&field.as_str()) {
                match value {
                    JsonValue::String(s) => {
                        where_clauses.push(format!("r.{} = '{}'", field, s.replace("'", "''")));
                    }
                    JsonValue::Number(n) => {
                        where_clauses.push(format!("r.{} = {}", field, n));
                    }
                    JsonValue::Bool(b) => {
                        where_clauses.push(format!(
                            "r.{} = {}",
                            field,
                            if *b { "TRUE" } else { "FALSE" }
                        ));
                    }
                    _ => {
                        where_clauses.push(format!(
                            "r.{} = '{}'",
                            field,
                            value.to_string().replace("'", "''")
                        ));
                    }
                }
            } else {
                // Entity-specific field
                match value {
                    JsonValue::String(s) => {
                        where_clauses.push(format!("e.{} = '{}'", field, s.replace("'", "''")));
                    }
                    JsonValue::Number(n) => {
                        where_clauses.push(format!("e.{} = {}", field, n));
                    }
                    JsonValue::Bool(b) => {
                        where_clauses.push(format!(
                            "e.{} = {}",
                            field,
                            if *b { "TRUE" } else { "FALSE" }
                        ));
                    }
                    _ => {
                        where_clauses.push(format!(
                            "e.{} = '{}'",
                            field,
                            value.to_string().replace("'", "''")
                        ));
                    }
                }
            }
        }

        // Construct the query
        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("AND {}", where_clauses.join(" AND "))
        };

        let query = format!(
            "SELECT r.*, e.* 
             FROM entities_registry r
             JOIN {} e ON r.uuid = e.uuid
             WHERE r.entity_type = $1 {}
             LIMIT {} OFFSET {}",
            table_name, where_clause, limit, offset
        );

        let rows = sqlx::query(&query)
            .bind(entity_type)
            .fetch_all(&self.pool)
            .await?;

        // Convert rows to DynamicEntity objects (similar to get_all_by_type)
        let mut entities = Vec::new();
        for row in rows {
            let mut field_data = HashMap::new();

            // Add system fields from registry
            field_data.insert(
                "uuid".to_string(),
                JsonValue::String(row.get::<Uuid, _>("uuid").to_string()),
            );
            field_data.insert(
                "entity_type".to_string(),
                JsonValue::String(row.get::<String, _>("entity_type")),
            );
            field_data.insert(
                "path".to_string(),
                JsonValue::String(row.get::<String, _>("path")),
            );
            // Add other system fields

            // Add custom fields from the entity table
            for field in &class_def_arc.fields {
                let field_name = &field.name;
                if let Ok(value) = row.try_get::<JsonValue, _>(field_name.as_str()) {
                    field_data.insert(field_name.clone(), value);
                }
            }

            entities.push(DynamicEntity {
                entity_type: entity_type.to_string(),
                field_data,
                definition: class_def_arc.clone(),
            });
        }

        Ok(entities)
    }

    async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        let table_name = format!("entity_{}", entity_type.to_lowercase());

        let query = format!(
            "SELECT COUNT(*) as count
             FROM entities_registry r
             JOIN {} e ON r.uuid = e.uuid
             WHERE r.entity_type = $1",
            table_name
        );

        let row = sqlx::query(&query)
            .bind(entity_type)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = row.get("count");
        Ok(count)
    }
}
