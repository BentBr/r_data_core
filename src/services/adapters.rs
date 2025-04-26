use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use crate::api::admin::class_definitions::repository::ClassDefinitionRepository;
use crate::entity::class::definition::ClassDefinition;
use crate::entity::class::repository_trait::ClassDefinitionRepositoryTrait;
use crate::error::Result;

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

// More adapters can be added here as needed for other repositories
