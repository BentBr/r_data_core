use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;
use time::OffsetDateTime;

use crate::api::admin::class_definitions::repository::ClassDefinitionRepository;
use crate::entity::class::definition::ClassDefinition;
use crate::entity::class::repository_trait::ClassDefinitionRepositoryTrait;
use crate::error::{Error, Result};
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::dynamic_entity::repository::DynamicEntityRepository;
use crate::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
use std::sync::Arc;
use serde_json::Value as JsonValue;

/// Repository adapter for ClassDefinitionRepository
pub struct ClassDefinitionRepositoryAdapter {
    inner: ClassDefinitionRepository,
}

impl ClassDefinitionRepositoryAdapter {
    /// Create a new adapter that wraps the repository implementation
    pub fn new(repository: ClassDefinitionRepository) -> Self {
        Self { inner: repository }
    }
}

#[async_trait]
impl ClassDefinitionRepositoryTrait for ClassDefinitionRepositoryAdapter {
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<ClassDefinition>> {
        log::debug!("ClassDefinitionRepositoryAdapter::list called");
        self.inner.list(limit, offset).await
    }

    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<ClassDefinition>> {
        log::debug!(
            "ClassDefinitionRepositoryAdapter::get_by_uuid called with uuid: {}",
            uuid
        );
        self.inner.get_by_uuid(uuid).await
    }

    async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<ClassDefinition>> {
        log::debug!(
            "ClassDefinitionRepositoryAdapter::get_by_entity_type called with entity_type: {}",
            entity_type
        );
        self.inner.get_by_entity_type(entity_type).await
    }

    async fn create(&self, definition: &ClassDefinition) -> Result<Uuid> {
        log::debug!("ClassDefinitionRepositoryAdapter::create called");
        log::debug!("Definition entity_type: {}", definition.entity_type);
        log::debug!("Definition display_name: {}", definition.display_name);
        log::debug!(
            "Definition schema properties: {:?}",
            definition.schema.properties
        );
        log::debug!("Definition UUID: {}", definition.uuid);
        log::debug!("Definition created_by: {}", definition.created_by);
        log::debug!("Definition fields count: {}", definition.fields.len());

        // Ensure schema is properly initialized
        let result = self.inner.create(definition).await;

        if let Err(ref e) = result {
            log::error!("Error creating class definition in adapter: {:?}", e);
        } else {
            log::debug!("Class definition created successfully in adapter");
        }

        result
    }

    async fn update(&self, uuid: &Uuid, definition: &ClassDefinition) -> Result<()> {
        log::debug!(
            "ClassDefinitionRepositoryAdapter::update called with uuid: {}",
            uuid
        );
        self.inner.update(uuid, definition).await
    }

    async fn delete(&self, uuid: &Uuid) -> Result<()> {
        log::debug!(
            "ClassDefinitionRepositoryAdapter::delete called with uuid: {}",
            uuid
        );
        self.inner.delete(uuid).await
    }

    async fn apply_schema(&self, schema_sql: &str) -> Result<()> {
        log::debug!("ClassDefinitionRepositoryAdapter::apply_schema called");
        self.inner.apply_schema(schema_sql).await
    }

    async fn update_entity_view_for_class_definition(
        &self,
        class_definition: &ClassDefinition,
    ) -> Result<()> {
        log::debug!(
            "ClassDefinitionRepositoryAdapter::update_entity_view_for_class_definition called"
        );
        self.inner
            .update_entity_view_for_class_definition(class_definition)
            .await
    }

    async fn check_view_exists(&self, view_name: &str) -> Result<bool> {
        log::debug!(
            "ClassDefinitionRepositoryAdapter::check_view_exists called with view_name: {}",
            view_name
        );
        self.inner.check_view_exists(view_name).await
    }

    async fn get_view_columns_with_types(
        &self,
        view_name: &str,
    ) -> Result<HashMap<String, String>> {
        log::debug!("ClassDefinitionRepositoryAdapter::get_view_columns_with_types called with view_name: {}", view_name);
        self.inner.get_view_columns_with_types(view_name).await
    }

    async fn count_view_records(&self, view_name: &str) -> Result<i64> {
        log::debug!(
            "ClassDefinitionRepositoryAdapter::count_view_records called with view_name: {}",
            view_name
        );
        self.inner.count_view_records(view_name).await
    }

    async fn cleanup_unused_entity_view(&self) -> Result<()> {
        log::debug!("ClassDefinitionRepositoryAdapter::cleanup_unused_entity_view called");
        self.inner.cleanup_unused_entity_view().await
    }
}

/// Dynamic entity repository adapter
pub struct DynamicEntityRepositoryAdapter {
    inner: DynamicEntityRepository,
}

impl DynamicEntityRepositoryAdapter {
    /// Create a new adapter
    pub fn new(inner: DynamicEntityRepository) -> Self {
        Self { inner }
    }

    /// Adapt a concrete repository implementation to a trait
    pub fn from_repository(repository: DynamicEntityRepository) -> Self {
        Self { inner: repository }
    }
}

#[async_trait]
impl DynamicEntityRepositoryTrait for DynamicEntityRepositoryAdapter {
    async fn create(&self, entity: &DynamicEntity) -> Result<()> {
        self.inner.create(entity).await
    }

    async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        self.inner.update(entity).await
    }

    async fn get_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<Option<DynamicEntity>> {
        // Implement this by querying for a specific entity
        // This is a placeholder; we should be using the inner repository's get_by_uuid or similar
        log::debug!("get_by_type called for type {} and uuid {}", entity_type, uuid);
        
        // Currently the repository doesn't have a method for this, so we'll need to implement it
        // or use a workaround like filtering by UUID
        let mut filters = HashMap::new();
        filters.insert("uuid".to_string(), JsonValue::String(uuid.to_string()));
        
        match self.inner.filter_entities(entity_type, &filters, 1, 0).await {
            Ok(entities) => {
                if let Some(entity) = entities.into_iter().next() {
                    Ok(Some(entity))
                } else {
                    Ok(None)
                }
            },
            Err(e) => Err(e),
        }
    }

    async fn get_all_by_type(&self, entity_type: &str, limit: i64, offset: i64) -> Result<Vec<DynamicEntity>> {
        // Use an empty filter to get all entities of a type
        let filters = HashMap::new();
        log::debug!("get_all_by_type called for type {} with limit {} and offset {}", entity_type, limit, offset);
        
        // Safely handle errors
        match self.inner.filter_entities(entity_type, &filters, limit, offset).await {
            Ok(entities) => {
                log::debug!("Found {} entities of type {}", entities.len(), entity_type);
                Ok(entities)
            },
            Err(e) => {
                // If we get a NotFound error about the class definition, return an empty list
                match e {
                    Error::NotFound(msg) if msg.contains("Class definition") => {
                        log::warn!("Class definition for type {} not found, returning empty list", entity_type);
                        Ok(Vec::new())
                    },
                    _ => {
                        log::error!("Error getting entities of type {}: {}", entity_type, e);
                        Err(e)
                    }
                }
            }
        }
    }

    async fn delete_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<()> {
        // Implementation would depend on how the concrete repository handles deletions
        // For now, we'll need a workaround since the repository doesn't have this method
        
        // First, check if the entity exists
        match self.get_by_type(entity_type, uuid).await {
            Ok(Some(_)) => {
                // TODO: Implement actual deletion logic here
                // For now, just return Ok assuming it worked
                log::warn!("delete_by_type called for type {} and uuid {}, but deletion not implemented", entity_type, uuid);
                Ok(())
            },
            Ok(None) => {
                // Entity doesn't exist, so nothing to delete
                Ok(())
            },
            Err(e) => Err(e),
        }
    }

    async fn filter_entities(
        &self,
        entity_type: &str,
        filters: &HashMap<String, JsonValue>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DynamicEntity>> {
        // Implement by delegating to the inner repository
        self.inner.filter_entities(entity_type, filters, limit, offset).await
    }

    async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        // Use an empty filter to get all entities of a type, then count them
        log::debug!("count_entities called for type {}", entity_type);
        
        // First check if class definition exists
        let empty_filters = HashMap::new();
        
        // Using filter_entities with a limit of 0 just to check if the entity type exists
        match self.inner.filter_entities(entity_type, &empty_filters, 0, 0).await {
            Ok(_) => {
                // If we can filter entities, entity type exists, count via SQL
                // For tables that exist but are empty, this will return 0
                
                // Get count using SQL directly - the actual implementation would depend
                // on DynamicEntityRepository implementation details
                let table_name = format!("entity_{}", entity_type.to_lowercase());
                
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
                .fetch_one(&self.inner.pool)
                .await?;
                
                if !table_exists {
                    return Ok(0);
                }
                
                let query = format!("SELECT COUNT(*) FROM \"{}\"", table_name);
                let count: i64 = sqlx::query_scalar(&query)
                    .fetch_one(&self.inner.pool)
                    .await?;
                
                Ok(count)
            },
            Err(e) => {
                // If we get a NotFound error about the class definition, return 0
                match e {
                    Error::NotFound(msg) if msg.contains("Class definition") => {
                        log::warn!("Class definition for type {} not found, returning count of 0", entity_type);
                        Ok(0)
                    },
                    _ => {
                        log::error!("Error counting entities of type {}: {}", entity_type, e);
                        Err(e)
                    }
                }
            }
        }
    }
}

/// Repository adapter for AdminUserRepository
pub struct AdminUserRepositoryAdapter {
    inner: crate::entity::admin_user::repository::AdminUserRepository,
}

impl AdminUserRepositoryAdapter {
    /// Create a new adapter that wraps the repository implementation
    pub fn new(repository: crate::entity::admin_user::repository::AdminUserRepository) -> Self {
        Self { inner: repository }
    }
}

#[async_trait]
impl crate::entity::admin_user::repository_trait::AdminUserRepositoryTrait for AdminUserRepositoryAdapter {
    async fn find_by_username_or_email(&self, username_or_email: &str) -> Result<Option<crate::entity::admin_user::AdminUser>> {
        log::debug!("AdminUserRepositoryAdapter::find_by_username_or_email called with username_or_email: {}", username_or_email);
        self.inner.find_by_username_or_email(username_or_email).await
    }

    async fn find_by_uuid(&self, uuid: &Uuid) -> Result<Option<crate::entity::admin_user::AdminUser>> {
        log::debug!("AdminUserRepositoryAdapter::find_by_uuid called with uuid: {}", uuid);
        self.inner.find_by_uuid(uuid).await
    }

    async fn update_last_login(&self, uuid: &Uuid) -> Result<()> {
        log::debug!("AdminUserRepositoryAdapter::update_last_login called with uuid: {}", uuid);
        self.inner.update_last_login(uuid).await
    }

    async fn create_admin_user<'a>(&self, 
        username: &str,
        email: &str,
        password: &str,
        first_name: &str,
        last_name: &str,
        role: Option<&'a str>,
        is_active: bool,
        creator_uuid: Uuid,
    ) -> Result<Uuid> {
        log::debug!("AdminUserRepositoryAdapter::create_admin_user called with username: {}", username);
        self.inner.create_admin_user(
            username, 
            email, 
            password, 
            first_name, 
            last_name, 
            role, 
            is_active, 
            creator_uuid
        ).await
    }

    async fn update_admin_user(&self, user: &crate::entity::admin_user::AdminUser) -> Result<()> {
        log::debug!("AdminUserRepositoryAdapter::update_admin_user called for user uuid: {}", user.uuid);
        self.inner.update_admin_user(user).await
    }

    async fn delete_admin_user(&self, uuid: &Uuid) -> Result<()> {
        log::debug!("AdminUserRepositoryAdapter::delete_admin_user called with uuid: {}", uuid);
        self.inner.delete_admin_user(uuid).await
    }

    async fn list_admin_users(&self, limit: i64, offset: i64) -> Result<Vec<crate::entity::admin_user::AdminUser>> {
        log::debug!("AdminUserRepositoryAdapter::list_admin_users called with limit: {}, offset: {}", limit, offset);
        self.inner.list_admin_users(limit, offset).await
    }
}
