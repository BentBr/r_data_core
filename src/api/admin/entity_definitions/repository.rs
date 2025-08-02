use crate::db::PgPoolExtension;
use crate::entity::entity_definition::definition::EntityDefinition;
use crate::entity::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use crate::entity::field::types::FieldType;
use crate::error::{Error, Result};
use async_trait::async_trait;
use log;
use serde_json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::collections::HashSet;
use uuid::Uuid;

pub struct EntityDefinitionRepository {
    db_pool: PgPool,
}

impl EntityDefinitionRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Get the database connection pool
    fn get_pool(&self) -> &PgPool {
        &self.db_pool
    }

    /// Check if a view exists in the database
    pub async fn check_view_exists(&self, view_name: &str) -> Result<bool> {
        // First check for views
        let view_exists = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT FROM information_schema.views
                WHERE table_schema = 'public' AND table_name = $1
            ) as "exists!"
            "#,
            view_name
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(Error::Database)?
        .exists;

        if view_exists {
            return Ok(true);
        }

        // If view doesn't exist, check for table
        let table_exists = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = $1
            ) as "exists!"
            "#,
            view_name
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(Error::Database)?
        .exists;

        Ok(table_exists)
    }

    /// Get columns and their types for a view
    pub async fn get_view_columns_with_types(
        &self,
        view_name: &str,
    ) -> Result<HashMap<String, String>> {
        let columns = sqlx::query!(
            r#"
            SELECT column_name, data_type
            FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1
            "#,
            view_name
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        let mut column_types = HashMap::new();
        for column in columns {
            if let (Some(name), Some(data_type)) = (column.column_name, column.data_type) {
                column_types.insert(name, data_type);
            }
        }

        Ok(column_types)
    }

    /// Count records in a view or table
    pub async fn count_view_records(&self, table_name: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {}", table_name))
            .fetch_one(&self.db_pool)
            .await
            .map_err(Error::Database)?;

        Ok(count)
    }

    /// Get all column names for a table or view
    async fn get_existing_table_columns(&self, table_name: &str) -> Result<Vec<String>> {
        let columns = sqlx::query!(
            r#"
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1
            "#,
            table_name
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        Ok(columns
            .into_iter()
            .filter_map(|col| col.column_name)
            .collect())
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

    /// Delete entities from the entities_registry by entity type
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
}

#[async_trait]
impl EntityDefinitionRepositoryTrait for EntityDefinitionRepository {
    /// List all entity definitions with pagination
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<EntityDefinition>> {
        self.db_pool
            .repository_with_table::<EntityDefinition>("entity_definitions")
            .list(None, Some("entity_type ASC"), Some(limit), Some(offset))
            .await
    }

    async fn count(&self) -> Result<i64> {
        self.db_pool
            .repository_with_table::<EntityDefinition>("entity_definitions")
            .count(None)
            .await
    }

    /// Get a entity definition by UUID
    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<EntityDefinition>> {
        // Use custom query with explicit type casting
        let entity_def = sqlx::query!(
            r#"
            SELECT
                uuid, entity_type, display_name, description, group_name,
                allow_children, icon, field_definitions as "field_definitions: serde_json::Value",
                created_at, updated_at,
                created_by as "created_by: Uuid", updated_by,
                published, version
            FROM entity_definitions
            WHERE uuid = $1
            "#,
            uuid
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        if let Some(entity_def) = entity_def {
            // Create schema
            let mut properties = HashMap::new();
            properties.insert(
                "entity_type".to_string(),
                serde_json::Value::String(entity_def.entity_type.clone()),
            );
            let schema = crate::entity::entity_definition::schema::Schema::new(properties);

            // Convert to EntityDefinition
            let definition = EntityDefinition {
                uuid: entity_def.uuid,
                entity_type: entity_def.entity_type,
                display_name: entity_def.display_name,
                description: entity_def.description,
                group_name: entity_def.group_name,
                allow_children: entity_def.allow_children,
                icon: entity_def.icon,
                fields: serde_json::from_value(entity_def.field_definitions)
                    .map_err(|e| Error::Serialization(e))?,
                schema,
                created_at: entity_def.created_at,
                updated_at: entity_def.updated_at,
                created_by: entity_def.created_by,
                updated_by: entity_def.updated_by,
                published: entity_def.published,
                version: entity_def.version,
            };
            Ok(Some(definition))
        } else {
            Ok(None)
        }
    }

    /// Get a entity definition by entity type
    async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<EntityDefinition>> {
        let entity_def = sqlx::query!(
            r#"
            SELECT
                uuid, entity_type, display_name, description, group_name,
                allow_children, icon, field_definitions as "field_definitions: serde_json::Value",
                created_at, updated_at,
                created_by as "created_by: Uuid", updated_by,
                published, version
            FROM entity_definitions
            WHERE entity_type = $1
            "#,
            entity_type
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        if let Some(entity_def) = entity_def {
            // Create schema
            let mut properties = HashMap::new();
            properties.insert(
                "entity_type".to_string(),
                serde_json::Value::String(entity_def.entity_type.clone()),
            );
            let schema = crate::entity::entity_definition::schema::Schema::new(properties);

            // Convert to EntityDefinition
            Ok(Some(EntityDefinition {
                uuid: entity_def.uuid,
                entity_type: entity_def.entity_type,
                display_name: entity_def.display_name,
                description: entity_def.description,
                group_name: entity_def.group_name,
                allow_children: entity_def.allow_children,
                icon: entity_def.icon,
                fields: serde_json::from_value(entity_def.field_definitions)
                    .map_err(|e| Error::Serialization(e))?,
                schema,
                created_at: entity_def.created_at,
                updated_at: entity_def.updated_at,
                created_by: entity_def.created_by,
                updated_by: entity_def.updated_by,
                published: entity_def.published,
                version: entity_def.version,
            }))
        } else {
            Ok(None)
        }
    }

    /// Create a new entity definition
    async fn create(&self, definition: &EntityDefinition) -> Result<Uuid> {
        // We need a custom implementation because the general repository requires a path field
        // that entity definitions don't have

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
        log::debug!("Creating entity definition with UUID: {}", uuid);
        log::debug!("Entity type: {}", entity_type);
        log::debug!(
            "Created by: {} (type: {})",
            created_by,
            std::any::type_name_of_val(&created_by)
        );
        log::debug!("Fields: {}", fields);
        log::debug!("Schema properties: {:?}", definition.schema.properties);

        // SQL query with named parameters for clarity
        let query = "INSERT INTO entity_definitions
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
                log::error!("Database error creating entity definition: {}", e);
                Error::Database(e)
            })?;

        // Explicitly create or update the entity table and view is already done by the trigger
        // No need to call it here as it will be handled by the database

        Ok(result)
    }

    /// Update an existing entity definition
    async fn update(&self, uuid: &Uuid, definition: &EntityDefinition) -> Result<()> {
        // Custom implementation for entity definitions that doesn't require a path field

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
        let query = "UPDATE entity_definitions SET
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

    /// Delete a entity definition
    async fn delete(&self, uuid: &Uuid) -> Result<()> {
        // First, get the entity definition to get the entity type
        let entity_definition_result = self.get_by_uuid(uuid).await?;

        if let Some(entity_definition) = entity_definition_result {
            let table_name = entity_definition.get_table_name();

            // Drop the entity table if it exists
            let table_exists = self.check_view_exists(&table_name).await?;
            if table_exists {
                // Also drop any relation tables if they exist
                for field in &entity_definition.fields {
                    if let FieldType::ManyToMany = field.field_type {
                        let relation_table_name = format!(
                            "rel_{}_{}",
                            entity_definition.entity_type.to_lowercase(),
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
            self.delete_from_entities_registry(&entity_definition.entity_type)
                .await?;

            // Finally, delete the definition from the entity_definitions table
            sqlx::query("DELETE FROM entity_definitions WHERE uuid = $1")
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
        // Split the SQL into individual statements and execute each one separately
        let statements: Vec<&str> = schema_sql
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for statement in statements {
            if !statement.trim().is_empty() {
                log::debug!("Executing SQL statement: {}", statement);
                sqlx::query(statement)
                    .execute(&self.db_pool)
                    .await
                    .map_err(Error::Database)?;
            }
        }
        Ok(())
    }

    /// Update entity table and view for entity definition
    async fn update_entity_view_for_entity_definition(
        &self,
        entity_definition: &EntityDefinition,
    ) -> Result<()> {
        log::info!(
            "Updating entity table and view for entity type: {}",
            entity_definition.entity_type
        );

        // Generate the complete schema SQL including indexes
        let schema_sql = entity_definition.generate_schema_sql();

        log::debug!("Generated schema SQL: {}", schema_sql);

        // Apply the schema using the Rust-generated SQL
        self.apply_schema(&schema_sql).await?;

        log::info!(
            "Successfully created/updated table and view for entity type {}",
            entity_definition.entity_type
        );

        Ok(())
    }

    /// Cleanup unused entity tables
    async fn cleanup_unused_entity_view(&self) -> Result<()> {
        // Get all entity definitions
        let entity_definitions = self.list(1000, 0).await?;

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

        // Identify tables without a corresponding entity definition
        let defined_tables: HashSet<String> = entity_definitions
            .iter()
            .map(|def| def.get_table_name())
            .collect();

        for row in tables {
            if let Some(table_name) = row.table_name {
                if !defined_tables.contains(&table_name) {
                    // Table has no corresponding entity definition, drop it
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

    /// Get columns and their types for a view - delegates to implementation in EntityDefinitionRepository
    async fn get_view_columns_with_types(
        &self,
        view_name: &str,
    ) -> Result<HashMap<String, String>> {
        EntityDefinitionRepository::get_view_columns_with_types(self, view_name).await
    }

    /// Count records in a view or table - delegates to implementation in EntityDefinitionRepository
    async fn count_view_records(&self, view_name: &str) -> Result<i64> {
        EntityDefinitionRepository::count_view_records(self, view_name).await
    }

    /// Check if a view or table exists in the database - delegates to implementation in EntityDefinitionRepository
    async fn check_view_exists(&self, view_name: &str) -> Result<bool> {
        EntityDefinitionRepository::check_view_exists(self, view_name).await
    }
}
