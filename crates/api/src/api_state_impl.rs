#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::PgPool;
use std::sync::Arc;

use crate::api_state::ApiStateTrait;
use r_data_core_core::cache::CacheManager;
use r_data_core_services::{AdminUserService, ApiKeyService, DynamicEntityService, EntityDefinitionService, PermissionSchemeService, WorkflowService};
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;

/// Shared application state
/// 
/// This is the concrete implementation of ApiStateTrait used in the main application.
/// It holds all the services and dependencies needed by the API routes.
pub struct ApiState {
    /// Database connection pool
    pub db_pool: PgPool,

    /// API configuration (includes JWT secret and expiration)
    pub api_config: r_data_core_core::config::ApiConfig,

    /// Cache manager
    pub cache_manager: Arc<CacheManager>,

    /// API Key service
    pub api_key_service: ApiKeyService,

    /// Admin User service
    pub admin_user_service: AdminUserService,

    /// Entity Definition service
    pub entity_definition_service: EntityDefinitionService,

    /// Dynamic Entity service
    pub dynamic_entity_service: Option<Arc<DynamicEntityService>>,

    /// Workflow service (data import/export workflows)
    pub workflow_service: WorkflowService,

    /// Permission scheme service
    pub permission_scheme_service: PermissionSchemeService,

    /// Queue client for producing jobs
    pub queue: Arc<ApalisRedisQueue>,
}

// Implement ApiStateTrait for ApiState to allow API crate routes to use it
impl ApiStateTrait for ApiState {
    fn db_pool(&self) -> &PgPool {
        &self.db_pool
    }

    fn jwt_secret(&self) -> &str {
        &self.api_config.jwt_secret
    }

    fn api_key_service_ref(&self) -> &dyn std::any::Any {
        &self.api_key_service
    }

    fn permission_scheme_service_ref(&self) -> &dyn std::any::Any {
        &self.permission_scheme_service
    }

    fn api_config_ref(&self) -> &dyn std::any::Any {
        &self.api_config
    }

    fn entity_definition_service_ref(&self) -> &dyn std::any::Any {
        &self.entity_definition_service
    }

    fn dynamic_entity_service_ref(&self) -> Option<&dyn std::any::Any> {
        self.dynamic_entity_service.as_ref().map(|s| s as &dyn std::any::Any)
    }

    fn cache_manager_ref(&self) -> &dyn std::any::Any {
        &*self.cache_manager
    }

    fn workflow_service_ref(&self) -> &dyn std::any::Any {
        &self.workflow_service
    }

    fn queue_ref(&self) -> &dyn std::any::Any {
        &*self.queue
    }
}

