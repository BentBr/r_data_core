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

        // Use the view for this entity type
        let view_name = format!("{}_view", entity_type);

        // Query the entity from the view
        let query = format!("SELECT * FROM {} WHERE uuid = $1", view_name);

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

    // Create a new entity using the columnar approach
    pub async fn create_entity(
        &self,
        entity_type: &str,
        fields: HashMap<String, Value>,
    ) -> Result<Uuid> {
        // Get class definition to understand entity structure
        let class_def_result = sqlx::query!(
            r#"
            SELECT entity_type, display_name, description, 
                   group_name, allow_children, icon 
            FROM class_definitions
            WHERE entity_type = $1
            "#,
            entity_type
        )
        .fetch_optional(&self.db_pool)
        .await?;

        let class_def = if let Some(row) = class_def_result {
            // In a real implementation, you'd load the field definitions too
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

        // Generate UUID for new entity
        let context = uuid::ContextV7::new();
        let ts = uuid::timestamp::Timestamp::now(&context);
        let uuid = Uuid::new_v7(ts);

        // If no path is provided, generate a default one
        let path = fields
            .get("path")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("/{}/{}", entity_type.to_lowercase(), uuid));

        // Create a transaction
        let mut tx = self.db_pool.begin().await?;

        // First, insert the entity into the entities_registry table
        sqlx::query!(
            r#"
            INSERT INTO entities_registry
            (uuid, entity_type, path, created_at, updated_at, created_by, updated_by, published, version)
            VALUES
            ($1, $2, $3, NOW(), NOW(), $4, $4, $5, 1)
            "#,
            uuid,
            entity_type,
            path,
            fields.get("created_by").and_then(|v| serde_json::from_value::<Uuid>(v.clone()).ok()),
            fields.get("published").and_then(|v| v.as_bool()).unwrap_or(false)
        )
        .execute(&mut *tx)
        .await?;

        // Now insert custom fields into the entity-specific table
        // Get column names for this table to ensure we only insert valid fields
        let columns_result = sqlx::query!(
            r#"
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1
            "#,
            table_name
        )
        .fetch_all(&mut *tx)
        .await?;

        // Build a list of valid columns for this table
        let valid_columns: Vec<String> = columns_result
            .into_iter()
            .map(|row| row.column_name.unwrap_or_default().to_lowercase())
            .collect();

        // Filter fields to only include those that are valid columns in the entity table
        // Exclude fields that are now part of the entities_registry table
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

        let mut entity_fields = HashMap::new();
        for (name, value) in &fields {
            if !registry_fields.contains(&name.as_str())
                && valid_columns.contains(&name.to_lowercase())
            {
                entity_fields.insert(name.clone(), value.clone());
            }
        }

        if !entity_fields.is_empty() {
            // We need to handle custom fields by constructing SQL manually
            let mut manual_columns = Vec::new();
            let mut manual_values = Vec::new();

            // Add UUID first
            manual_columns.push("uuid".to_string());
            manual_values.push(format!("'{}'", uuid));

            // Add all entity fields that match valid columns
            for (name, value) in entity_fields {
                if valid_columns.contains(&name.to_lowercase()) {
                    manual_columns.push(name);

                    // Convert values to strings with proper SQL escaping
                    match value {
                        Value::String(s) => {
                            manual_values.push(format!("'{}'", s.replace("'", "''")))
                        }
                        Value::Number(n) => manual_values.push(n.to_string()),
                        Value::Bool(b) => manual_values.push(if b {
                            "TRUE".to_string()
                        } else {
                            "FALSE".to_string()
                        }),
                        Value::Null => manual_values.push("NULL".to_string()),
                        _ => manual_values
                            .push(format!("'{}'", value.to_string().replace("'", "''"))), // For complex types
                    }
                }
            }

            // Build and execute the INSERT query
            let manual_query = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                manual_columns.join(", "),
                manual_values.join(", ")
            );

            sqlx::query(&manual_query).execute(&mut *tx).await?;
        } else {
            // If there are no custom fields, just insert the UUID
            sqlx::query(&format!("INSERT INTO {} (uuid) VALUES ($1)", table_name))
                .bind(uuid)
                .execute(&mut *tx)
                .await?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(uuid)
    }

    // Update an existing entity using the columnar approach
    pub async fn update_entity(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        fields: HashMap<String, Value>,
    ) -> Result<()> {
        // Get class definition to understand entity structure
        let class_def_result = sqlx::query!(
            r#"
            SELECT entity_type, display_name, description, 
                   group_name, allow_children, icon 
            FROM class_definitions
            WHERE entity_type = $1
            "#,
            entity_type
        )
        .fetch_optional(&self.db_pool)
        .await?;

        let class_def = if let Some(row) = class_def_result {
            // In a real implementation, you'd load the field definitions too
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

        // Start a transaction
        let mut tx = self.db_pool.begin().await?;

        // 1. Update the entities_registry table
        let mut registry_fields = HashMap::new();

        // Extract registry fields
        if let Some(path) = fields.get("path") {
            if let Some(path_str) = path.as_str() {
                registry_fields.insert("path", path_str.to_string());
            }
        }

        if let Some(published) = fields.get("published") {
            if let Some(published_bool) = published.as_bool() {
                registry_fields.insert("published", published_bool.to_string());
            }
        }

        if let Some(updated_by) = fields.get("updated_by") {
            if let Some(updated_by_str) = updated_by.as_str() {
                registry_fields.insert("updated_by", format!("'{}'", updated_by_str));
            }
        }

        // Always update the updated_at timestamp and increment version
        sqlx::query!(
            r#"
            UPDATE entities_registry
            SET updated_at = NOW(), version = version + 1
            WHERE uuid = $1 AND entity_type = $2
            "#,
            uuid,
            entity_type
        )
        .execute(&mut *tx)
        .await?;

        // Add additional registry fields if provided
        if !registry_fields.is_empty() {
            let set_clauses: Vec<String> = registry_fields
                .iter()
                .map(|(k, v)| format!("{} = {}", k, v))
                .collect();

            let update_query = format!(
                "UPDATE entities_registry SET {} WHERE uuid = $1 AND entity_type = $2",
                set_clauses.join(", ")
            );

            sqlx::query(&update_query)
                .bind(uuid)
                .bind(entity_type)
                .execute(&mut *tx)
                .await?;
        }

        // 2. Update the entity-specific table
        // Get column names for this table to ensure we only update valid fields
        let columns_result = sqlx::query!(
            r#"
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1
            "#,
            table_name
        )
        .fetch_all(&mut *tx)
        .await?;

        let valid_columns: Vec<String> = columns_result
            .into_iter()
            .map(|row| row.column_name.unwrap_or_default().to_lowercase())
            .collect();

        // Extract fields to update in the entity-specific table
        // Exclude fields that are now part of the entities_registry table
        let registry_field_names = [
            "entity_type",
            "path",
            "created_at",
            "updated_at",
            "created_by",
            "updated_by",
            "published",
            "version",
        ];

        let mut update_fields = HashMap::new();
        for (name, value) in &fields {
            if !registry_field_names.contains(&name.as_str())
                && valid_columns.contains(&name.to_lowercase())
            {
                update_fields.insert(name.clone(), value.clone());
            }
        }

        // If we have fields to update, construct the SQL
        if !update_fields.is_empty() {
            let set_clauses: Vec<String> = update_fields
                .iter()
                .map(|(k, v)| {
                    // Very simplified value formatting - in production you would use proper parameterization
                    let value_str = match v {
                        Value::String(s) => format!("'{}'", s.replace("'", "''")),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => {
                            if *b {
                                "TRUE".to_string()
                            } else {
                                "FALSE".to_string()
                            }
                        }
                        Value::Null => "NULL".to_string(),
                        _ => "NULL".to_string(), // For complex types, would need more handling
                    };

                    format!("{} = {}", k, value_str)
                })
                .collect();

            let update_query = format!(
                "UPDATE {} SET {} WHERE uuid = $1",
                table_name,
                set_clauses.join(", ")
            );

            sqlx::query(&update_query)
                .bind(uuid)
                .execute(&mut *tx)
                .await?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(())
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
