#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod admin_user_repository;
pub mod admin_user_repository_trait;
pub mod dynamic_entity_mapper;
pub mod dynamic_entity_repository;
pub mod dynamic_entity_repository_trait;
pub mod dynamic_entity_utils;
pub mod dynamic_entity_versioning;
pub mod entity_definition_repository;
pub mod entity_definition_versioning_repository;
pub mod permission_scheme_repository;
pub mod refresh_token_repository;
pub mod refresh_token_repository_trait;
pub mod repository;
pub mod settings_repository;
pub mod version_repository;
pub mod workflow_versioning_repository;
pub mod workflow_repository;
pub mod workflow_repository_trait;
pub use r_data_core_core as core;

// Re-export commonly used types
pub use admin_user_repository::{AdminUserRepository, ApiKeyRepository};
pub use admin_user_repository_trait::{
    AdminUserRepositoryTrait, ApiKeyRepositoryTrait, is_key_valid,
};
pub use dynamic_entity_repository::DynamicEntityRepository;
pub use dynamic_entity_repository_trait::DynamicEntityRepositoryTrait;
pub use entity_definition_repository::EntityDefinitionRepository;
pub use entity_definition_versioning_repository::{
    EntityDefinitionVersioningRepository, EntityDefinitionVersionMeta, EntityDefinitionVersionPayload,
};
pub use permission_scheme_repository::PermissionSchemeRepository;
pub use refresh_token_repository::RefreshTokenRepository;
pub use refresh_token_repository_trait::RefreshTokenRepositoryTrait;
pub use repository::{EntityRepository, PgPoolExtension};
pub use settings_repository::SystemSettingsRepository;
pub use version_repository::VersionRepository;
pub use workflow_versioning_repository::{WorkflowVersioningRepository, WorkflowVersionMeta, WorkflowVersionPayload};
pub use workflow_repository::WorkflowRepository;
pub use workflow_repository_trait::WorkflowRepositoryTrait;
