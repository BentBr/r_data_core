use async_trait::async_trait;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core_core::error::Error;
use r_data_core_core::error::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

mod reads;
mod schema;
mod writes;

/// Repository for entity definition operations
pub struct EntityDefinitionRepository {
    pub(crate) db_pool: PgPool,
}

impl EntityDefinitionRepository {
    /// Create a new entity definition repository
    #[must_use]
    pub const fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Check if a view exists in the database
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn check_view_exists(&self, view_name: &str) -> Result<bool> {
        // First check for views
        let view_exists = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT FROM information_schema.views
                WHERE table_schema = current_schema() AND table_name = $1
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
                WHERE table_schema = current_schema() AND table_name = $1
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
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_view_columns_with_types(
        &self,
        view_name: &str,
    ) -> Result<HashMap<String, String>> {
        let columns = sqlx::query!(
            "
            SELECT column_name, data_type
            FROM information_schema.columns
            WHERE table_schema = current_schema() AND table_name = $1
            ",
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
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn count_view_records(&self, table_name: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table_name}"))
            .fetch_one(&self.db_pool)
            .await
            .map_err(Error::Database)?;

        Ok(count)
    }

    /// Delete entities from the `entities_registry` by entity type
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn delete_from_entities_registry(&self, entity_type: &str) -> Result<()> {
        sqlx::query!(
            "
            DELETE FROM entities_registry WHERE entity_type = $1
            ",
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
        reads::list(self, limit, offset).await
    }

    async fn count(&self) -> Result<i64> {
        reads::count(self).await
    }

    /// Get a entity definition by UUID
    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<EntityDefinition>> {
        reads::get_by_uuid(self, uuid).await
    }

    /// Get a entity definition by entity type
    async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<EntityDefinition>> {
        reads::get_by_entity_type(self, entity_type).await
    }

    /// Create a new entity definition
    async fn create(&self, definition: &EntityDefinition) -> Result<Uuid> {
        writes::create(self, definition).await
    }

    /// Update an existing entity definition
    async fn update(&self, uuid: &Uuid, definition: &EntityDefinition) -> Result<()> {
        writes::update(self, uuid, definition).await
    }

    /// Delete a entity definition
    async fn delete(&self, uuid: &Uuid) -> Result<()> {
        writes::delete(self, uuid).await
    }

    /// Apply schema SQL to database
    async fn apply_schema(&self, schema_sql: &str) -> Result<()> {
        schema::apply_schema(self, schema_sql).await
    }

    /// Update entity table and view for entity definition
    async fn update_entity_view_for_entity_definition(
        &self,
        entity_definition: &EntityDefinition,
    ) -> Result<()> {
        schema::update_entity_view_for_entity_definition(self, entity_definition).await
    }

    /// Cleanup unused entity tables
    async fn cleanup_unused_entity_view(&self) -> Result<()> {
        schema::cleanup_unused_entity_view(self).await
    }

    /// Get columns and their types for a view - delegates to implementation in `EntityDefinitionRepository`
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn get_view_columns_with_types(
        &self,
        view_name: &str,
    ) -> Result<HashMap<String, String>> {
        Self::get_view_columns_with_types(self, view_name).await
    }

    /// Count records in a view or table - delegates to implementation in `EntityDefinitionRepository`
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn count_view_records(&self, view_name: &str) -> Result<i64> {
        Self::count_view_records(self, view_name).await
    }

    /// Check if a view or table exists in the database - delegates to implementation in `EntityDefinitionRepository`
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn check_view_exists(&self, view_name: &str) -> Result<bool> {
        Self::check_view_exists(self, view_name).await
    }
}
