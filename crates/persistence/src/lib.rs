#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod admin_user_repository;
pub mod admin_user_repository_trait;
pub mod api_key_repository;
pub mod dynamic_entity_mapper;
pub mod dynamic_entity_public_repository;
pub mod dynamic_entity_query_repository;
pub mod dynamic_entity_query_repository_trait;
pub mod dynamic_entity_repository;
pub mod dynamic_entity_repository_trait;
pub mod dynamic_entity_utils;
pub mod dynamic_entity_versioning;
pub mod entity_definition_repository;
pub mod entity_definition_versioning_repository;
pub mod entity_definition_versioning_repository_trait;
pub mod permission_scheme_repository;
pub mod permission_scheme_repository_trait;
pub mod refresh_token_repository;
pub mod refresh_token_repository_trait;
pub mod repository;
pub mod settings_repository;
pub mod settings_repository_trait;
pub mod version_repository;
pub mod version_repository_trait;
pub mod workflow_repository;
pub mod workflow_repository_trait;
pub mod workflow_versioning_repository;
pub mod workflow_versioning_repository_trait;
pub use r_data_core_core as core;

// Re-export commonly used types
pub use admin_user_repository::AdminUserRepository;
pub use admin_user_repository_trait::{
    is_key_valid, AdminUserRepositoryTrait, ApiKeyRepositoryTrait,
};
pub use api_key_repository::ApiKeyRepository;
pub use dynamic_entity_public_repository::DynamicEntityPublicRepository;
pub use dynamic_entity_query_repository::DynamicEntityQueryRepository;
pub use dynamic_entity_query_repository_trait::DynamicEntityQueryRepositoryTrait;
pub use dynamic_entity_repository::DynamicEntityRepository;
pub use dynamic_entity_repository_trait::DynamicEntityRepositoryTrait;
pub use entity_definition_repository::EntityDefinitionRepository;
pub use entity_definition_versioning_repository::{
    EntityDefinitionVersionMeta, EntityDefinitionVersionPayload,
    EntityDefinitionVersioningRepository,
};
pub use entity_definition_versioning_repository_trait::EntityDefinitionVersioningRepositoryTrait;
pub use permission_scheme_repository::PermissionSchemeRepository;
pub use permission_scheme_repository_trait::PermissionSchemeRepositoryTrait;
pub use refresh_token_repository::RefreshTokenRepository;
pub use refresh_token_repository_trait::RefreshTokenRepositoryTrait;
pub use repository::{EntityRepository, PgPoolExtension};
pub use settings_repository::SystemSettingsRepository;
pub use settings_repository_trait::SettingsRepositoryTrait;
pub use version_repository::{EntityVersionMeta, EntityVersionPayload, VersionRepository};
pub use version_repository_trait::VersionRepositoryTrait;
pub use workflow_repository::{get_provider_config, WorkflowRepository};
pub use workflow_repository_trait::WorkflowRepositoryTrait;
pub use workflow_versioning_repository::{
    WorkflowVersionMeta, WorkflowVersionPayload, WorkflowVersioningRepository,
};
pub use workflow_versioning_repository_trait::WorkflowVersioningRepositoryTrait;
