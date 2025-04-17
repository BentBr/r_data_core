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

        // Get column names for this table to ensure we only insert valid fields
        let columns_result = sqlx::query!(
            r#"
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1
            "#,
            table_name
        )
        .fetch_all(&self.db_pool)
        .await?;

        let valid_columns: Vec<String> = columns_result
            .into_iter()
            .map(|row| row.column_name.unwrap_or_default().to_lowercase())
            .collect();

        // Build column names and placeholders for SQL query
        let mut column_names = vec!["uuid".to_string(), "path".to_string()];
        let mut placeholders = vec!["$1".to_string(), "$2".to_string()];
        let mut values: Vec<Value> = vec![Value::String(uuid.to_string()), Value::String(path)];

        // Add system fields if not provided
        let now = chrono::Utc::now().to_rfc3339();

        if !fields.contains_key("created_at") {
            column_names.push("created_at".to_string());
            placeholders.push(format!("${}", column_names.len()));
            values.push(Value::String(now.clone()));
        }

        if !fields.contains_key("updated_at") {
            column_names.push("updated_at".to_string());
            placeholders.push(format!("${}", column_names.len()));
            values.push(Value::String(now.clone()));
        }

        if !fields.contains_key("version") {
            column_names.push("version".to_string());
            placeholders.push(format!("${}", column_names.len()));
            values.push(Value::Number(1.into()));
        }

        if !fields.contains_key("published") {
            column_names.push("published".to_string());
            placeholders.push(format!("${}", column_names.len()));
            values.push(Value::Bool(false));
        }

        // Add all other provided fields that are valid columns
        for (key, value) in fields {
            // Skip fields we've already handled
            if key == "uuid"
                || key == "path"
                || key == "created_at"
                || key == "updated_at"
                || key == "version"
                || key == "published"
            {
                continue;
            }

            // Only include valid columns
            if valid_columns.contains(&key.to_lowercase()) {
                column_names.push(key);
                placeholders.push(format!("${}", column_names.len()));
                values.push(value);
            }
        }

        // Build and execute the insert query
        let query = format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING uuid",
            table_name,
            column_names.join(", "),
            placeholders.join(", ")
        );

        // Execute the query with all values
        let mut query_builder = sqlx::query(&query);
        for value in values {
            query_builder = query_builder.bind(value);
        }

        let result = query_builder.fetch_one(&self.db_pool).await?;

        let inserted_uuid: Uuid = result.get("uuid");

        Ok(inserted_uuid)
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

        // Get column names for this table to ensure we only update valid fields
        let columns_result = sqlx::query!(
            r#"
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1
            "#,
            table_name
        )
        .fetch_all(&self.db_pool)
        .await?;

        let valid_columns: Vec<String> = columns_result
            .into_iter()
            .map(|row| row.column_name.unwrap_or_default().to_lowercase())
            .collect();

        // Build set clauses for SQL query
        let mut set_clauses = Vec::new();
        let mut values: Vec<Value> = Vec::new();

        // Always update updated_at
        let now = chrono::Utc::now().to_rfc3339();
        set_clauses.push(format!("updated_at = ${}", set_clauses.len() + 1));
        values.push(Value::String(now));

        // Increment version
        set_clauses.push(format!("version = version + 1"));

        // Add all other provided fields that are valid columns
        for (key, value) in fields {
            // Skip immutable fields
            if key == "uuid" || key == "created_at" || key == "created_by" {
                continue;
            }

            // Only include valid columns
            if valid_columns.contains(&key.to_lowercase()) {
                set_clauses.push(format!("{} = ${}", key, set_clauses.len() + 1));
                values.push(value);
            }
        }

        // Build and execute the update query
        let query = format!(
            "UPDATE {} SET {} WHERE uuid = ${}",
            table_name,
            set_clauses.join(", "),
            values.len() + 1
        );

        // Execute the query with all values
        let mut query_builder = sqlx::query(&query);
        for value in values {
            query_builder = query_builder.bind(value);
        }
        query_builder = query_builder.bind(uuid);

        let result = query_builder.execute(&self.db_pool).await?;

        if result.rows_affected() == 0 {
            return Err(crate::error::Error::NotFound(format!(
                "Entity with UUID {} not found",
                uuid
            )));
        }

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
