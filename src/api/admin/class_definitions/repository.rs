use crate::db::PgPoolExtension;
use crate::entity::ClassDefinition;
use crate::error::Result;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ClassDefinitionRepository {
    db_pool: PgPool,
}

impl ClassDefinitionRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<ClassDefinition>> {
        self.db_pool
            .repository_with_table::<ClassDefinition>("class_definitions")
            .list(None, Some("entity_type ASC"), Some(limit), Some(offset))
            .await
    }

    pub async fn get_by_uuid(&self, uuid: &Uuid) -> Result<ClassDefinition> {
        self.db_pool
            .repository_with_table::<ClassDefinition>("class_definitions")
            .get_by_uuid(uuid)
            .await
    }

    pub async fn create(&self, definition: &ClassDefinition) -> Result<Uuid> {
        self.db_pool
            .repository_with_table::<ClassDefinition>("class_definitions")
            .create(definition)
            .await
    }

    pub async fn update(&self, uuid: &Uuid, definition: &ClassDefinition) -> Result<()> {
        self.db_pool
            .repository_with_table::<ClassDefinition>("class_definitions")
            .update(uuid, definition)
            .await
    }

    pub async fn delete(&self, uuid: &Uuid) -> Result<()> {
        self.db_pool
            .repository_with_table::<ClassDefinition>("class_definitions")
            .delete(uuid)
            .await
    }

    pub async fn apply_schema(&self, schema_sql: &str) -> Result<()> {
        sqlx::query(schema_sql).execute(&self.db_pool).await?;
        Ok(())
    }

    pub async fn check_table_exists(&self, table_name: &str) -> Result<bool> {
        let result: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = $1
            )",
        )
        .bind(table_name.to_lowercase())
        .fetch_one(&self.db_pool)
        .await?;

        Ok(result.0)
    }

    pub async fn count_table_records(&self, table_name: &str) -> Result<i64> {
        let result: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table_name))
            .fetch_one(&self.db_pool)
            .await?;

        Ok(result.0)
    }

    pub async fn delete_from_entity_registry(&self, entity_type: &str) -> Result<()> {
        sqlx::query("DELETE FROM entities WHERE name = $1")
            .bind(entity_type)
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }
}
