#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod adapters;
pub mod admin_user;
pub mod api_key;
pub mod bootstrap;
pub mod dynamic_entity;
pub mod entity_definition;
pub mod permission_scheme;
pub mod settings;
pub mod version;
pub mod worker;
pub mod workflow;

// Re-exports
pub use adapters::{
    AdminUserRepositoryAdapter, ApiKeyRepositoryAdapter, DynamicEntityRepositoryAdapter,
    EntityDefinitionRepositoryAdapter,
};
pub use admin_user::AdminUserService;
pub use api_key::ApiKeyService;
pub use bootstrap::{init_cache_manager, init_logger_with_default, init_pg_pool};
pub use dynamic_entity::DynamicEntityService;
pub use entity_definition::{EntityDefinitionService, ServiceEntityFieldInfo};
pub use permission_scheme::PermissionSchemeService;
pub use settings::SettingsService;
pub use version::{VersionMetaWithName, VersionService};
pub use worker::compute_reconcile_actions;
pub use workflow::{WorkflowRepositoryAdapter, WorkflowService};
