use crate::db::PgPoolExtension;
use crate::entity::class::definition::ClassDefinition;
use crate::entity::class::repository_trait::ClassDefinitionRepositoryTrait;
use crate::entity::field::types::FieldType;
use crate::error::{Error, Result};
use async_trait::async_trait;
use log;
use serde_json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::collections::HashSet;
use uuid::Uuid;

pub struct ClassDefinitionRepository {
    db_pool: PgPool,
}

impl ClassDefinitionRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Get available columns from the entities_registry table
    async fn get_available_registry_columns(&self) -> Result<Vec<String>> {
        let columns = sqlx::query!(
            r#"
            SELECT column_name 
            FROM information_schema.columns
            WHERE table_schema = 'public'
            AND table_name = 'entities_registry'
            "#
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        let available_columns: Vec<String> = columns
            .into_iter()
            .filter_map(|col| col.column_name)
            .collect();

        Ok(available_columns)
    }
}

#[async_trait]
impl ClassDefinitionRepositoryTrait for ClassDefinitionRepository {
    /// List all class definitions with pagination
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<ClassDefinition>> {
        self.db_pool
            .repository_with_table::<ClassDefinition>("class_definitions")
            .list(None, Some("entity_type ASC"), Some(limit), Some(offset))
            .await
    }

    /// Get a class definition by UUID
    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<ClassDefinition>> {
        // Use custom query with explicit type casting
        let class_def = sqlx::query!(
            r#"
            SELECT 
                uuid, entity_type, display_name, description, group_name, 
                allow_children, icon, field_definitions as "field_definitions: serde_json::Value",
                created_at, updated_at, 
                created_by as "created_by: Uuid", updated_by,
                published, version
            FROM class_definitions
            WHERE uuid = $1
            "#,
            uuid
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        if let Some(class_def) = class_def {
            // Create schema
            let mut properties = HashMap::new();
            properties.insert(
                "entity_type".to_string(),
                serde_json::Value::String(class_def.entity_type.clone()),
            );
            let schema = crate::entity::class::schema::Schema::new(properties);

            // Convert to ClassDefinition
            let definition = ClassDefinition {
                uuid: class_def.uuid,
                entity_type: class_def.entity_type,
                display_name: class_def.display_name,
                description: class_def.description,
                group_name: class_def.group_name,
                allow_children: class_def.allow_children,
                icon: class_def.icon,
                fields: serde_json::from_value(class_def.field_definitions)
                    .map_err(|e| Error::Serialization(e))?,
                schema,
                created_at: class_def.created_at,
                updated_at: class_def.updated_at,
                created_by: class_def.created_by,
                updated_by: class_def.updated_by,
                published: class_def.published,
                version: class_def.version,
            };
            Ok(Some(definition))
        } else {
            Ok(None)
        }
    }

    /// Get a class definition by entity type
    async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<ClassDefinition>> {
        let class_def = sqlx::query!(
            r#"
            SELECT 
                uuid, entity_type, display_name, description, group_name, 
                allow_children, icon, field_definitions as "field_definitions: serde_json::Value",
                created_at, updated_at, 
                created_by as "created_by: Uuid", updated_by,
                published, version
            FROM class_definitions
            WHERE entity_type = $1
            "#,
            entity_type
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        if let Some(class_def) = class_def {
            // Create schema
            let mut properties = HashMap::new();
            properties.insert(
                "entity_type".to_string(),
                serde_json::Value::String(class_def.entity_type.clone()),
            );
            let schema = crate::entity::class::schema::Schema::new(properties);

            // Convert to ClassDefinition
            Ok(Some(ClassDefinition {
                uuid: class_def.uuid,
                entity_type: class_def.entity_type,
                display_name: class_def.display_name,
                description: class_def.description,
                group_name: class_def.group_name,
                allow_children: class_def.allow_children,
                icon: class_def.icon,
                fields: serde_json::from_value(class_def.field_definitions)
                    .map_err(|e| Error::Serialization(e))?,
                schema,
                created_at: class_def.created_at,
                updated_at: class_def.updated_at,
                created_by: class_def.created_by,
                updated_by: class_def.updated_by,
                published: class_def.published,
                version: class_def.version,
            }))
        } else {
            Ok(None)
        }
    }

    /// Create a new class definition
    async fn create(&self, definition: &ClassDefinition) -> Result<Uuid> {
        // We need a custom implementation because the general repository requires a path field
        // that class definitions don't have

        // Build the SQL fields and values
        let uuid = definition.uuid;
        let entity_type = &definition.entity_type;
        let display_name = &definition.display_name;
        let description = definition.description.as_ref();
        let group_name = definition.group_name.as_ref();
        let allow_children = definition.allow_children;
        let icon = definition.icon.as_ref();
        let fields =
            serde_json::to_value(&definition.fields).map_err(|e| Error::Serialization(e))?;
        let created_at = definition.created_at;
        let updated_at = definition.updated_at;
        let created_by: Uuid = definition.created_by;
        let updated_by = definition.updated_by;
        let published = definition.published;
        let version = definition.version;

        // Log values for debugging
        log::debug!("Creating class definition with UUID: {}", uuid);
        log::debug!("Entity type: {}", entity_type);
        log::debug!(
            "Created by: {} (type: {})",
            created_by,
            std::any::type_name_of_val(&created_by)
        );

        // SQL query with named parameters for clarity
        let query = "INSERT INTO class_definitions 
                    (uuid, entity_type, display_name, description, group_name, allow_children, 
                     icon, field_definitions, created_at, updated_at, created_by, updated_by, 
                     published, version) 
                    VALUES 
                    ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) 
                    RETURNING uuid";

        let result = sqlx::query_scalar::<_, Uuid>(query)
            .bind(uuid)
            .bind(entity_type)
            .bind(display_name)
            .bind(description)
            .bind(group_name)
            .bind(allow_children)
            .bind(icon)
            .bind(fields)
            .bind(created_at)
            .bind(updated_at)
            .bind(created_by)
            .bind(updated_by)
            .bind(published)
            .bind(version)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| {
                log::error!("Database error creating class definition: {}", e);
                Error::Database(e)
            })?;

        // Explicitly create or update the entity table and view is already done by the trigger
        // No need to call it here as it will be handled by the database

        Ok(result)
    }

    /// Update an existing class definition
    async fn update(&self, uuid: &Uuid, definition: &ClassDefinition) -> Result<()> {
        // Custom implementation for class definitions that doesn't require a path field

        // Build the SQL fields and values
        let entity_type = &definition.entity_type;
        let display_name = &definition.display_name;
        let description = definition.description.as_ref();
        let group_name = definition.group_name.as_ref();
        let allow_children = definition.allow_children;
        let icon = definition.icon.as_ref();
        let fields =
            serde_json::to_value(&definition.fields).map_err(|e| Error::Serialization(e))?;
        let updated_at = definition.updated_at;
        let updated_by = definition.updated_by;
        let published = definition.published;
        let version = definition.version;

        // SQL query
        let query = "UPDATE class_definitions SET 
                    entity_type = $1, 
                    display_name = $2, 
                    description = $3, 
                    group_name = $4, 
                    allow_children = $5, 
                    icon = $6, 
                    field_definitions = $7, 
                    updated_at = $8, 
                    updated_by = $9, 
                    published = $10, 
                    version = $11 
                    WHERE uuid = $12";

        sqlx::query(query)
            .bind(entity_type)
            .bind(display_name)
            .bind(description)
            .bind(group_name)
            .bind(allow_children)
            .bind(icon)
            .bind(fields)
            .bind(updated_at)
            .bind(updated_by)
            .bind(published)
            .bind(version)
            .bind(uuid)
            .execute(&self.db_pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    /// Delete a class definition
    async fn delete(&self, uuid: &Uuid) -> Result<()> {
        // First, get the class definition to get the entity type
        let class_definition_result = self.get_by_uuid(uuid).await?;

        if let Some(class_definition) = class_definition_result {
            let table_name = class_definition.get_table_name();

            // Drop the entity table if it exists
            let table_exists = self.check_view_exists(&table_name).await?;
            if table_exists {
                // Also drop any relation tables if they exist
                for field in &class_definition.fields {
                    if let FieldType::ManyToMany = field.field_type {
                        let relation_table_name = format!(
                            "rel_{}_{}",
                            class_definition.entity_type.to_lowercase(),
                            field.name.to_lowercase()
                        );

                        let rel_table_exists = self.check_view_exists(&relation_table_name).await?;
                        if rel_table_exists {
                            log::info!("Dropping relation table: {}", relation_table_name);
                            let drop_rel_sql =
                                format!("DROP TABLE IF EXISTS {} CASCADE", relation_table_name);
                            sqlx::query(&drop_rel_sql)
                                .execute(&self.db_pool)
                                .await
                                .map_err(Error::Database)?;
                        }
                    }
                }

                // Drop the entity table
                log::info!("Dropping entity table: {}", table_name);
                let drop_entity_sql = format!("DROP TABLE IF EXISTS {} CASCADE", table_name);
                sqlx::query(&drop_entity_sql)
                    .execute(&self.db_pool)
                    .await
                    .map_err(Error::Database)?;
            }

            // Remove the entity from the custom entities registry
            self.delete_from_entities_registry(&class_definition.entity_type)
                .await?;

            // Finally, delete the definition from the class_definitions table
            sqlx::query("DELETE FROM class_definitions WHERE uuid = $1")
                .bind(uuid)
                .execute(&self.db_pool)
                .await
                .map_err(Error::Database)?;

            Ok(())
        } else {
            Err(Error::NotFound(format!(
                "Class definition with UUID {} not found",
                uuid
            )))
        }
    }

    /// Apply schema SQL to database
    async fn apply_schema(&self, schema_sql: &str) -> Result<()> {
        sqlx::query(schema_sql)
            .execute(&self.db_pool)
            .await
            .map_err(Error::Database)?;
        Ok(())
    }

    /// Update entity table and view for class definition
    async fn update_entity_view_for_class_definition(
        &self,
        class_definition: &ClassDefinition,
    ) -> Result<()> {
        // This handles creating tables, adding and dropping columns, and updating views
        let sql = format!("SELECT create_entity_table_and_view($1)");

        log::info!(
            "Updating entity table and view for entity type: {}",
            class_definition.entity_type
        );

        // Add transaction to ensure atomicity
        let mut tx = self.db_pool.begin().await.map_err(Error::Database)?;

        let result = sqlx::query(&sql)
            .bind(&class_definition.entity_type)
            .execute(&mut *tx)
            .await;

        match result {
            Ok(_) => {
                log::info!(
                    "Successfully created/updated table and view for entity type {}",
                    class_definition.entity_type
                );

                // Commit the transaction only if successful
                tx.commit().await.map_err(|e| {
                    log::error!("Failed to commit transaction: {}", e);
                    Error::Database(e)
                })?;

                Ok(())
            }
            Err(e) => {
                // If an error occurs, the transaction will be rolled back automatically
                log::error!(
                    "Failed to create/update table and view for entity type {}: {}",
                    class_definition.entity_type,
                    e
                );

                // Provide more context in the error message
                Err(Error::Database(e))
            }
        }
    }

    /// Check if a table exists
    async fn check_view_exists(&self, table_name: &str) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public'
                AND table_name = $1
            ) as "exists!: bool"
            "#,
            table_name
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        Ok(result.exists)
    }

    /// Get view columns with their types
    async fn get_view_columns_with_types(
        &self,
        view_name: &str,
    ) -> Result<HashMap<String, String>> {
        let rows = sqlx::query!(
            r#"
            SELECT column_name, data_type
            FROM information_schema.columns
            WHERE table_schema = 'public'
            AND table_name = $1
            "#,
            view_name
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        let mut columns = HashMap::new();
        for row in rows {
            if let (Some(column_name), Some(data_type)) = (row.column_name, row.data_type) {
                columns.insert(column_name, data_type);
            }
        }

        Ok(columns)
    }

    /// Count records in a table
    async fn count_view_records(&self, table_name: &str) -> Result<i64> {
        let count_sql = format!("SELECT COUNT(*) FROM {}", table_name);
        let count = sqlx::query_scalar::<_, i64>(&count_sql)
            .fetch_one(&self.db_pool)
            .await
            .map_err(Error::Database)?;

        Ok(count)
    }

    /// Cleanup unused entity tables
    async fn cleanup_unused_entity_view(&self) -> Result<()> {
        // Get all class definitions
        let class_definitions = self.list(1000, 0).await?;

        // Get all tables starting with "entity_"
        let tables = sqlx::query!(
            r#"
            SELECT table_name
            FROM information_schema.tables
            WHERE table_schema = 'public'
            AND table_name LIKE 'entity_%'
            "#
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        // Identify tables without a corresponding class definition
        let defined_tables: HashSet<String> = class_definitions
            .iter()
            .map(|def| def.get_table_name())
            .collect();

        for row in tables {
            if let Some(table_name) = row.table_name {
                if !defined_tables.contains(&table_name) {
                    // Table has no corresponding class definition, drop it
                    log::info!("Dropping orphaned entity table: {}", table_name);
                    let drop_sql = format!("DROP TABLE IF EXISTS {} CASCADE", table_name);

                    sqlx::query(&drop_sql)
                        .execute(&self.db_pool)
                        .await
                        .map_err(Error::Database)?;
                }
            }
        }

        Ok(())
    }
}

// Keep these helper methods
impl ClassDefinitionRepository {
    pub async fn delete_from_entities_registry(&self, entity_type: &str) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM entities_registry WHERE entity_type = $1
            "#,
            entity_type
        )
        .execute(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        Ok(())
    }

    /// Get existing columns for a table
    async fn get_existing_table_columns(&self, table_name: &str) -> Result<Vec<String>> {
        let columns = sqlx::query!(
            r#"
            SELECT column_name 
            FROM information_schema.columns
            WHERE table_schema = 'public'
            AND table_name = $1
            "#,
            table_name
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        let column_names: Vec<String> = columns
            .into_iter()
            .filter_map(|col| col.column_name.map(|name| name.to_lowercase()))
            .collect();

        Ok(column_names)
    }
}

// ... Keep the rest of your existing implementation for helper methods ...(existing code continues)
