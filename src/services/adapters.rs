use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::admin::entity_definitions::repository::EntityDefinitionRepository;
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::dynamic_entity::repository::DynamicEntityRepository;
use crate::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
use crate::entity::entity_definition::definition::EntityDefinition;
use crate::entity::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use crate::error::Result;
use crate::workflow::data::repository::WorkflowRepository;
use crate::workflow::data::repository_trait::WorkflowRepositoryTrait;
use serde_json::Value as JsonValue;
// Workflow data repository adapter
use crate::api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};
use crate::workflow::data::repository_trait::WorkflowRepositoryTrait as WorkflowRepositoryTraitDef;
use crate::workflow::data::WorkflowKind;

pub struct WorkflowRepositoryAdapter {
    inner: WorkflowRepository,
}

impl WorkflowRepositoryAdapter {
    pub fn new(inner: WorkflowRepository) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl WorkflowRepositoryTraitDef for WorkflowRepositoryAdapter {
    async fn list_all(&self) -> anyhow::Result<Vec<crate::workflow::data::Workflow>> {
        self.inner.list_all().await
    }

    async fn list_paginated(&self, limit: i64, offset: i64) -> anyhow::Result<Vec<crate::workflow::data::Workflow>> {
        self.inner.list_paginated(limit, offset).await
    }

    async fn count_all(&self) -> anyhow::Result<i64> {
        self.inner.count_all().await
    }

    async fn get_by_uuid(
        &self,
        uuid: Uuid,
    ) -> anyhow::Result<Option<crate::workflow::data::Workflow>> {
        self.inner.get_by_uuid(uuid).await
    }

    async fn create(&self, req: &CreateWorkflowRequest) -> anyhow::Result<Uuid> {
        self.inner.create(req).await
    }

    async fn update(&self, uuid: Uuid, req: &UpdateWorkflowRequest) -> anyhow::Result<()> {
        self.inner.update(uuid, req).await
    }

    async fn delete(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.inner.delete(uuid).await
    }

    async fn list_scheduled_consumers(&self) -> anyhow::Result<Vec<(Uuid, String)>> {
        self.inner.list_scheduled_consumers().await
    }

    async fn insert_run_queued(&self, workflow_uuid: Uuid, trigger_id: Uuid) -> anyhow::Result<Uuid> {
        self.inner
            .insert_run_queued(workflow_uuid, trigger_id)
            .await
    }

    async fn list_runs_paginated(
        &self,
        workflow_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<(Uuid, String, Option<String>, Option<String>, Option<i64>, Option<i64>)>, i64)> {
        self.inner
            .list_runs_paginated(workflow_uuid, limit, offset)
            .await
    }

    async fn list_run_logs_paginated(
        &self,
        run_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<(Vec<(Uuid, String, String, String, Option<serde_json::Value>)>, i64)> {
        self.inner
            .list_run_logs_paginated(run_uuid, limit, offset)
            .await
    }

    async fn run_exists(&self, run_uuid: Uuid) -> anyhow::Result<bool> {
        self.inner.run_exists(run_uuid).await
    }

    async fn list_all_runs_paginated(&self, limit: i64, offset: i64) -> anyhow::Result<(Vec<(Uuid, String, Option<String>, Option<String>, Option<i64>, Option<i64>)>, i64)> {
        self.inner.list_all_runs_paginated(limit, offset).await
    }

    async fn insert_run_log(&self, run_uuid: Uuid, level: &str, message: &str, meta: Option<serde_json::Value>) -> anyhow::Result<()> {
        self.inner.insert_run_log(run_uuid, level, message, meta).await
    }

    async fn insert_raw_items(&self, workflow_uuid: Uuid, run_uuid: Uuid, payloads: Vec<serde_json::Value>) -> anyhow::Result<i64> {
        self.inner.insert_raw_items(workflow_uuid, run_uuid, payloads).await
    }

    async fn count_raw_items_for_run(&self, run_uuid: Uuid) -> anyhow::Result<i64> {
        self.inner.count_raw_items_for_run(run_uuid).await
    }

    async fn mark_raw_items_processed(&self, run_uuid: Uuid) -> anyhow::Result<()> {
        self.inner.mark_raw_items_processed(run_uuid).await
    }

    async fn fetch_staged_raw_items(&self, run_uuid: Uuid, limit: i64) -> anyhow::Result<Vec<(Uuid, serde_json::Value)>> {
        self.inner.fetch_staged_raw_items(run_uuid, limit).await
    }

    async fn set_raw_item_status(&self, item_uuid: Uuid, status: &str, error: Option<&str>) -> anyhow::Result<()> {
        self.inner.set_raw_item_status(item_uuid, status, error).await
    }

    async fn mark_run_success(&self, run_uuid: Uuid, processed: i64, failed: i64) -> anyhow::Result<()> {
        self.inner.mark_run_success(run_uuid, processed, failed).await
    }

    async fn mark_run_failure(&self, run_uuid: Uuid, message: &str) -> anyhow::Result<()> {
        self.inner.mark_run_failure(run_uuid, message).await
    }
}

/// Repository adapter for EntityDefinitionRepository
pub struct EntityDefinitionRepositoryAdapter {
    inner: EntityDefinitionRepository,
}

impl EntityDefinitionRepositoryAdapter {
    /// Create a new adapter that wraps the repository implementation
    pub fn new(repository: EntityDefinitionRepository) -> Self {
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
            "EntityDefinitionRepositoryAdapter::get_by_uuid called with uuid: {}",
            uuid
        );
        self.inner.get_by_uuid(uuid).await
    }

    async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<EntityDefinition>> {
        log::debug!(
            "EntityDefinitionRepositoryAdapter::get_by_entity_type called with entity_type: {}",
            entity_type
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
            log::error!("Error creating entity definition in adapter: {:?}", e);
        } else {
            log::debug!("Entity definition created successfully in adapter");
        }

        result
    }

    async fn update(&self, uuid: &Uuid, definition: &EntityDefinition) -> Result<()> {
        log::debug!(
            "EntityDefinitionRepositoryAdapter::update called with uuid: {}",
            uuid
        );
        self.inner.update(uuid, definition).await
    }

    async fn delete(&self, uuid: &Uuid) -> Result<()> {
        log::debug!(
            "EntityDefinitionRepositoryAdapter::delete called with uuid: {}",
            uuid
        );
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
            "EntityDefinitionRepositoryAdapter::check_view_exists called with view_name: {}",
            view_name
        );
        self.inner.check_view_exists(view_name).await
    }

    async fn get_view_columns_with_types(
        &self,
        view_name: &str,
    ) -> Result<HashMap<String, String>> {
        log::debug!("EntityDefinitionRepositoryAdapter::get_view_columns_with_types called with view_name: {}", view_name);
        self.inner.get_view_columns_with_types(view_name).await
    }

    async fn count_view_records(&self, view_name: &str) -> Result<i64> {
        log::debug!(
            "EntityDefinitionRepositoryAdapter::count_view_records called with view_name: {}",
            view_name
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
