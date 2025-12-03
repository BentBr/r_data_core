#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_persistence::{DynamicEntityRepository, DynamicEntityRepositoryTrait};

/// Repository adapter for `EntityDefinitionRepository`
pub struct EntityDefinitionRepositoryAdapter {
    inner: EntityDefinitionRepository,
}

impl EntityDefinitionRepositoryAdapter {
    /// Create a new adapter that wraps the repository implementation
    #[must_use]
    pub const fn new(repository: EntityDefinitionRepository) -> Self {
        Self { inner: repository }
    }
}

#[async_trait]
impl EntityDefinitionRepositoryTrait for EntityDefinitionRepositoryAdapter {
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<EntityDefinition>> {
        log::debug!("EntityDefinitionRepositoryAdapter::list called");
        self.inner.list(limit, offset).await
    }

    async fn count(&self) -> Result<i64> {
        log::debug!("EntityDefinitionRepositoryAdapter::count called");
        self.inner.count().await
    }

    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<EntityDefinition>> {
        log::debug!(
            "EntityDefinitionRepositoryAdapter::get_by_uuid called with uuid: {uuid}"
        );
        self.inner.get_by_uuid(uuid).await
    }

    async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<EntityDefinition>> {
        log::debug!(
            "EntityDefinitionRepositoryAdapter::get_by_entity_type called with entity_type: {entity_type}"
        );
        self.inner.get_by_entity_type(entity_type).await
    }

    async fn create(&self, definition: &EntityDefinition) -> Result<Uuid> {
        log::debug!("EntityDefinitionRepositoryAdapter::create called");
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
            log::error!("Error creating entity definition in adapter: {e:?}");
        } else {
            log::debug!("Entity definition created successfully in adapter");
        }

        result
    }

    async fn update(&self, uuid: &Uuid, definition: &EntityDefinition) -> Result<()> {
        log::debug!("EntityDefinitionRepositoryAdapter::update called with uuid: {uuid}");
        self.inner.update(uuid, definition).await
    }

    async fn delete(&self, uuid: &Uuid) -> Result<()> {
        log::debug!("EntityDefinitionRepositoryAdapter::delete called with uuid: {uuid}");
        self.inner.delete(uuid).await
    }

    async fn apply_schema(&self, schema_sql: &str) -> Result<()> {
        log::debug!("EntityDefinitionRepositoryAdapter::apply_schema called");
        self.inner.apply_schema(schema_sql).await
    }

    async fn update_entity_view_for_entity_definition(
        &self,
        entity_definition: &EntityDefinition,
    ) -> Result<()> {
        log::debug!(
            "EntityDefinitionRepositoryAdapter::update_entity_view_for_entity_definition called"
        );
        self.inner
            .update_entity_view_for_entity_definition(entity_definition)
            .await
    }

    async fn check_view_exists(&self, view_name: &str) -> Result<bool> {
        log::debug!(
            "EntityDefinitionRepositoryAdapter::check_view_exists called with view_name: {view_name}"
        );
        self.inner.check_view_exists(view_name).await
    }

    async fn get_view_columns_with_types(
        &self,
        view_name: &str,
    ) -> Result<HashMap<String, String>> {
        log::debug!("EntityDefinitionRepositoryAdapter::get_view_columns_with_types called with view_name: {view_name}");
        self.inner.get_view_columns_with_types(view_name).await
    }

    async fn count_view_records(&self, view_name: &str) -> Result<i64> {
        log::debug!(
            "EntityDefinitionRepositoryAdapter::count_view_records called with view_name: {view_name}"
        );
        self.inner.count_view_records(view_name).await
    }

    async fn cleanup_unused_entity_view(&self) -> Result<()> {
        log::debug!("EntityDefinitionRepositoryAdapter::cleanup_unused_entity_view called");
        self.inner.cleanup_unused_entity_view().await
    }
}

/// Dynamic entity repository adapter
pub struct DynamicEntityRepositoryAdapter {
    inner: DynamicEntityRepository,
}

impl DynamicEntityRepositoryAdapter {
    /// Create a new adapter
    #[must_use]
    pub const fn new(inner: DynamicEntityRepository) -> Self {
        Self { inner }
    }

    /// Adapt a concrete repository implementation to a trait
    #[must_use]
    pub const fn from_repository(repository: DynamicEntityRepository) -> Self {
        Self { inner: repository }
    }
}

#[async_trait]
impl DynamicEntityRepositoryTrait for DynamicEntityRepositoryAdapter {
    /// Create a new entity
    async fn create(&self, entity: &DynamicEntity) -> r_data_core_core::error::Result<()> {
        self.inner.create(entity).await
    }

    /// Update an existing entity
    async fn update(&self, entity: &DynamicEntity) -> r_data_core_core::error::Result<()> {
        self.inner.update(entity).await
    }

    /// Get a dynamic entity by type and UUID
    async fn get_by_type(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        exclusive_fields: Option<Vec<String>>,
    ) -> r_data_core_core::error::Result<Option<DynamicEntity>> {
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
    ) -> r_data_core_core::error::Result<Vec<DynamicEntity>> {
        self.inner
            .get_all_by_type(entity_type, limit, offset, exclusive_fields)
            .await
    }

    /// Delete an entity by type and UUID
    async fn delete_by_type(
        &self,
        entity_type: &str,
        uuid: &Uuid,
    ) -> r_data_core_core::error::Result<()> {
        self.inner.delete_by_type(entity_type, uuid).await
    }

    /// Filter entities by field values with advanced options
    async fn filter_entities(
        &self,
        entity_type: &str,
        params: &r_data_core_persistence::FilterEntitiesParams,
    ) -> r_data_core_core::error::Result<Vec<DynamicEntity>> {
        self.inner.filter_entities(entity_type, params).await
    }

    /// Count entities of a specific type
    async fn count_entities(&self, entity_type: &str) -> r_data_core_core::error::Result<i64> {
        self.inner.count_entities(entity_type).await
    }
}

/// Repository adapter for AdminUserRepository
pub struct AdminUserRepositoryAdapter {
    inner: r_data_core_persistence::AdminUserRepository,
}

impl AdminUserRepositoryAdapter {
    /// Create a new adapter that wraps the repository implementation
    #[must_use]
    pub const fn new(repository: r_data_core_persistence::AdminUserRepository) -> Self {
        Self { inner: repository }
    }
}

#[async_trait]
impl r_data_core_persistence::AdminUserRepositoryTrait for AdminUserRepositoryAdapter {
    async fn find_by_username_or_email(
        &self,
        username_or_email: &str,
    ) -> r_data_core_core::error::Result<Option<r_data_core_core::admin_user::AdminUser>> {
        log::debug!("AdminUserRepositoryAdapter::find_by_username_or_email called with username_or_email: {username_or_email}");
        self.inner.find_by_username_or_email(username_or_email).await
    }

    async fn find_by_uuid(
        &self,
        uuid: &Uuid,
    ) -> r_data_core_core::error::Result<Option<r_data_core_core::admin_user::AdminUser>> {
        log::debug!("AdminUserRepositoryAdapter::find_by_uuid called with uuid: {uuid}");
        self.inner.find_by_uuid(uuid).await
    }

    async fn update_last_login(&self, uuid: &Uuid) -> r_data_core_core::error::Result<()> {
        log::debug!("AdminUserRepositoryAdapter::update_last_login called with uuid: {uuid}");
        self.inner.update_last_login(uuid).await
    }

    async fn create_admin_user<'a>(
        &self,
        params: &r_data_core_persistence::CreateAdminUserParams<'a>,
    ) -> r_data_core_core::error::Result<Uuid> {
        log::debug!(
            "AdminUserRepositoryAdapter::create_admin_user called with username: {}",
            params.username
        );
        self.inner.create_admin_user(params).await
    }

    async fn update_admin_user(
        &self,
        user: &r_data_core_core::admin_user::AdminUser,
    ) -> Result<()> {
        log::debug!(
            "AdminUserRepositoryAdapter::update_admin_user called for user uuid: {}",
            user.uuid
        );
        self.inner.update_admin_user(user).await
    }

    async fn delete_admin_user(&self, uuid: &Uuid) -> Result<()> {
        log::debug!("AdminUserRepositoryAdapter::delete_admin_user called with uuid: {uuid}");
        self.inner.delete_admin_user(uuid).await
    }

    async fn list_admin_users(
        &self,
        limit: i64,
        offset: i64,
    ) -> r_data_core_core::error::Result<Vec<r_data_core_core::admin_user::AdminUser>> {
        log::debug!(
            "AdminUserRepositoryAdapter::list_admin_users called with limit: {limit}, offset: {offset}"
        );
        self.inner.list_admin_users(limit, offset).await
    }
}
