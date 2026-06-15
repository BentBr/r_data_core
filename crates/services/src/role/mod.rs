#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod cache;
mod crud;
mod permissions;

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use r_data_core_core::cache::CacheManager;
use r_data_core_persistence::{RoleRepository, RoleRepositoryTrait};
use serde::{Deserialize, Serialize};

use crate::SystemLogService;

/// Cached user role UUIDs
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserRoles {
    role_uuids: Vec<Uuid>,
}

/// Cached merged permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MergedPermissions {
    permissions: Vec<String>,
}

/// Service for managing roles with caching
pub struct RoleService {
    pub(super) repository: Arc<dyn RoleRepositoryTrait>,
    pub(super) pool: PgPool,
    pub(super) cache_manager: Arc<CacheManager>,
    pub(super) cache_ttl: Option<u64>,
    pub(super) system_log: Option<Arc<SystemLogService>>,
}

impl RoleService {
    /// Create a new role service with a database pool
    ///
    /// This convenience constructor creates a default `RoleRepository` from the pool.
    /// For testing or custom implementations, use `new_with_repository`.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `cache_manager` - Cache manager for caching roles
    /// * `cache_ttl` - Optional cache TTL in seconds (uses `entity_definition_ttl` if None)
    #[must_use]
    pub fn new(pool: PgPool, cache_manager: Arc<CacheManager>, cache_ttl: Option<u64>) -> Self {
        Self {
            repository: Arc::new(RoleRepository::new(pool.clone())),
            pool,
            cache_manager,
            cache_ttl,
            system_log: None,
        }
    }

    /// Create a new role service with a custom repository implementation
    ///
    /// Use this constructor when you need to inject a mock repository for testing
    /// or provide a custom implementation.
    ///
    /// # Arguments
    /// * `repository` - Role repository implementation
    /// * `pool` - Database connection pool (needed for auxiliary operations like cache invalidation)
    /// * `cache_manager` - Cache manager for caching roles
    /// * `cache_ttl` - Optional cache TTL in seconds
    #[must_use]
    pub fn new_with_repository(
        repository: Arc<dyn RoleRepositoryTrait>,
        pool: PgPool,
        cache_manager: Arc<CacheManager>,
        cache_ttl: Option<u64>,
    ) -> Self {
        Self {
            repository,
            pool,
            cache_manager,
            cache_ttl,
            system_log: None,
        }
    }

    /// Set the system log service for audit logging
    #[must_use]
    pub fn with_system_log(mut self, log: Arc<SystemLogService>) -> Self {
        self.system_log = Some(log);
        self
    }
}
