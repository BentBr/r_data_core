use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, FromRow, PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::entity::class::definition::ClassDefinition;
use crate::entity::dynamic_entity::entity::DynamicEntity;
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
                chrono::DateTime::parse_from_rfc3339(s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now())
            })
            .unwrap_or_else(chrono::Utc::now);

        let updated_at = entity
            .field_data
            .get("updated_at")
            .and_then(|v| v.as_str())
            .map(|s| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now())
            })
            .unwrap_or_else(chrono::Utc::now);

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
                    if let Some(created_by) = row.created_by {
                        data.insert(
                            "created_by".to_string(),
                            JsonValue::String(created_by.to_string()),
                        );
                    }
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
