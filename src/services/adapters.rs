use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::admin::class_definitions::repository::ClassDefinitionRepository;
use crate::entity::class::definition::ClassDefinition;
use crate::entity::class::repository_trait::ClassDefinitionRepositoryTrait;
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::dynamic_entity::repository::DynamicEntityRepository;
use crate::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
use crate::error::{Result};
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
    /// Create a new entity
    async fn create(&self, entity: &DynamicEntity) -> Result<()> {
        self.inner.create(entity).await
    }

    /// Update an existing entity
    async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        self.inner.update(entity).await
    }

    /// Get a dynamic entity by type and UUID
    async fn get_by_type(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Option<DynamicEntity>> {
        self.inner
            .get_by_type(entity_type, uuid, exclusive_fields)
            .await
    }

    /// Get all entities of a specific type with pagination
    async fn get_all_by_type(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        self.inner
            .get_all_by_type(entity_type, limit, offset, exclusive_fields)
            .await
    }

    /// Delete an entity by type and UUID
    async fn delete_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<()> {
        self.inner.delete_by_type(entity_type, uuid).await
    }

    /// Filter entities by field values with advanced options
    async fn filter_entities(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        filters: Option<HashMap<String, JsonValue>>,
        search: Option<(String, Vec<String>)>,
        sort: Option<(String, String)>,
        fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        self.inner
            .filter_entities(entity_type, limit, offset, filters, search, sort, fields)
            .await
    }

    /// Count entities of a specific type
    async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        self.inner.count_entities(entity_type).await
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
impl crate::entity::admin_user::repository_trait::AdminUserRepositoryTrait
    for AdminUserRepositoryAdapter
{
    async fn find_by_username_or_email(
        &self,
        username_or_email: &str,
    ) -> Result<Option<crate::entity::admin_user::AdminUser>> {
        log::debug!("AdminUserRepositoryAdapter::find_by_username_or_email called with username_or_email: {}", username_or_email);
        self.inner
            .find_by_username_or_email(username_or_email)
            .await
    }

    async fn find_by_uuid(
        &self,
        uuid: &Uuid,
    ) -> Result<Option<crate::entity::admin_user::AdminUser>> {
        log::debug!(
            "AdminUserRepositoryAdapter::find_by_uuid called with uuid: {}",
            uuid
        );
        self.inner.find_by_uuid(uuid).await
    }

    async fn update_last_login(&self, uuid: &Uuid) -> Result<()> {
        log::debug!(
            "AdminUserRepositoryAdapter::update_last_login called with uuid: {}",
            uuid
        );
        self.inner.update_last_login(uuid).await
    }

    async fn create_admin_user<'a>(
        &self,
        username: &str,
        email: &str,
        password: &str,
        first_name: &str,
        last_name: &str,
        role: Option<&'a str>,
        is_active: bool,
        creator_uuid: Uuid,
    ) -> Result<Uuid> {
        log::debug!(
            "AdminUserRepositoryAdapter::create_admin_user called with username: {}",
            username
        );
        self.inner
            .create_admin_user(
                username,
                email,
                password,
                first_name,
                last_name,
                role,
                is_active,
                creator_uuid,
            )
            .await
    }

    async fn update_admin_user(&self, user: &crate::entity::admin_user::AdminUser) -> Result<()> {
        log::debug!(
            "AdminUserRepositoryAdapter::update_admin_user called for user uuid: {}",
            user.uuid
        );
        self.inner.update_admin_user(user).await
    }

    async fn delete_admin_user(&self, uuid: &Uuid) -> Result<()> {
        log::debug!(
            "AdminUserRepositoryAdapter::delete_admin_user called with uuid: {}",
            uuid
        );
        self.inner.delete_admin_user(uuid).await
    }

    async fn list_admin_users(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::entity::admin_user::AdminUser>> {
        log::debug!(
            "AdminUserRepositoryAdapter::list_admin_users called with limit: {}, offset: {}",
            limit,
            offset
        );
        self.inner.list_admin_users(limit, offset).await
    }
}
