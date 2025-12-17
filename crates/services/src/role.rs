#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::{Permission, Role};
use r_data_core_persistence::{
    AdminUserRepository, ApiKeyRepository, RoleRepository, RoleRepositoryTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::query_validation::{validate_list_query, FieldValidator, ValidatedListQuery};

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
    repository: Arc<dyn RoleRepositoryTrait>,
    pool: PgPool,
    cache_manager: Arc<CacheManager>,
    cache_ttl: Option<u64>,
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
        }
    }

    /// Generate cache key for role
    fn cache_key(uuid: &Uuid) -> String {
        format!("role:{uuid}")
    }

    /// Generate cache key for user roles
    fn user_roles_cache_key(user_uuid: &Uuid) -> String {
        format!("user_roles:{user_uuid}")
    }

    /// Generate cache key for API key roles
    fn api_key_roles_cache_key(api_key_uuid: &Uuid) -> String {
        format!("api_key_roles:{api_key_uuid}")
    }

    /// Generate cache key for merged user permissions
    fn user_permissions_cache_key(user_uuid: &Uuid) -> String {
        format!("user_permissions:{user_uuid}")
    }

    /// Generate cache key for merged API key permissions
    fn api_key_permissions_cache_key(api_key_uuid: &Uuid) -> String {
        format!("api_key_permissions:{api_key_uuid}")
    }

    /// Get merged permissions for a user with caching
    ///
    /// # Arguments
    /// * `user_uuid` - User UUID
    /// * `admin_user_repo` - Admin user repository
    ///
    /// # Returns
    /// Vector of merged permission strings
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_merged_permissions_for_user(
        &self,
        user_uuid: Uuid,
        admin_user_repo: &AdminUserRepository,
    ) -> Result<Vec<String>> {
        let cache_key = Self::user_permissions_cache_key(&user_uuid);

        // Try cache first
        if let Ok(Some(cached)) = self
            .cache_manager
            .get::<MergedPermissions>(&cache_key)
            .await
        {
            return Ok(cached.permissions);
        }

        // Load roles and merge permissions
        let roles = self.get_roles_for_user(user_uuid, admin_user_repo).await?;

        // Merge permissions from all roles
        let mut permission_set = HashSet::new();
        let mut merged_permissions = Vec::new();

        for role in &roles {
            // If role is super_admin, user has all permissions
            if role.super_admin {
                // Return a special marker or all permissions
                // For now, we'll let the permission checker handle super_admin
                continue;
            }

            let role_permissions = role.get_permissions_as_strings();
            for perm in role_permissions {
                if permission_set.insert(perm.clone()) {
                    merged_permissions.push(perm);
                }
            }
        }

        // Cache the merged permissions
        let ttl = self.cache_ttl;
        let cached = MergedPermissions {
            permissions: merged_permissions.clone(),
        };
        if let Err(e) = self.cache_manager.set(&cache_key, &cached, ttl).await {
            log::warn!("Failed to cache merged permissions for user {user_uuid}: {e}");
        }

        Ok(merged_permissions)
    }

    /// Get merged permissions for an API key with caching
    ///
    /// # Arguments
    /// * `api_key_uuid` - API key UUID
    /// * `api_key_repo` - API key repository
    ///
    /// # Returns
    /// Vector of merged permission strings (from all roles)
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_merged_permissions_for_api_key(
        &self,
        api_key_uuid: Uuid,
        api_key_repo: &ApiKeyRepository,
    ) -> Result<Vec<String>> {
        let cache_key = Self::api_key_permissions_cache_key(&api_key_uuid);

        // Try cache first
        if let Ok(Some(cached)) = self
            .cache_manager
            .get::<MergedPermissions>(&cache_key)
            .await
        {
            return Ok(cached.permissions);
        }

        // Load roles and merge permissions
        let roles = self
            .get_roles_for_api_key(api_key_uuid, api_key_repo)
            .await?;
        let mut permission_set = HashSet::new();
        let mut merged_permissions = Vec::new();

        for role in &roles {
            // If role is super_admin, API key has all permissions
            if role.super_admin {
                continue;
            }

            for permission in &role.permissions {
                let perm_str = format!("{}", permission.permission_type).to_lowercase();
                let perm_string = permission
                    .constraints
                    .as_ref()
                    .and_then(|c| c.get("path"))
                    .map_or_else(
                        || format!("{}:{}", permission.resource_type.as_str(), perm_str),
                        |path| {
                            format!(
                                "{}:{}:{}",
                                permission.resource_type.as_str(),
                                path,
                                perm_str
                            )
                        },
                    );

                if permission_set.insert(perm_string.clone()) {
                    merged_permissions.push(perm_string);
                }
            }
        }

        // Cache the merged permissions
        let ttl = self.cache_ttl;
        let cached = MergedPermissions {
            permissions: merged_permissions.clone(),
        };
        if let Err(e) = self.cache_manager.set(&cache_key, &cached, ttl).await {
            log::warn!("Failed to cache merged permissions for API key {api_key_uuid}: {e}");
        }

        Ok(merged_permissions)
    }

    /// Invalidate cached permissions for a user
    ///
    /// This invalidates both the roles cache and any cached merged permissions.
    ///
    /// # Arguments
    /// * `user_uuid` - User UUID
    pub async fn invalidate_user_permissions_cache(&self, user_uuid: &Uuid) {
        // Invalidate roles cache
        let cache_key = Self::user_roles_cache_key(user_uuid);
        if let Err(e) = self.cache_manager.delete(&cache_key).await {
            log::warn!("Failed to invalidate user roles cache {user_uuid}: {e}");
        }

        // Invalidate merged permissions cache
        let merged_cache_key = Self::user_permissions_cache_key(user_uuid);
        if let Err(e) = self.cache_manager.delete(&merged_cache_key).await {
            log::warn!("Failed to invalidate merged permissions cache for user {user_uuid}: {e}");
        }
    }

    /// Invalidate cached permissions for an API key
    ///
    /// # Arguments
    /// * `api_key_uuid` - API key UUID
    pub async fn invalidate_api_key_permissions_cache(&self, api_key_uuid: &Uuid) {
        // Invalidate merged permissions cache
        let cache_key = Self::api_key_permissions_cache_key(api_key_uuid);
        if let Err(e) = self.cache_manager.delete(&cache_key).await {
            log::warn!("Failed to invalidate API key permissions cache {api_key_uuid}: {e}");
        }
        // Also invalidate roles cache
        let roles_cache_key = Self::api_key_roles_cache_key(api_key_uuid);
        if let Err(e) = self.cache_manager.delete(&roles_cache_key).await {
            log::warn!("Failed to invalidate API key roles cache {api_key_uuid}: {e}");
        }
    }

    /// Invalidate all caches for users and API keys that reference a role
    ///
    /// This invalidates role caches, user role caches, API key role caches,
    /// and all merged permissions caches for affected users/API keys.
    ///
    /// # Arguments
    /// * `role_uuid` - Role UUID
    async fn invalidate_all_caches_for_role(&self, role_uuid: Uuid) {
        use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};

        // Invalidate role cache
        let role_cache_key = Self::cache_key(&role_uuid);
        if let Err(e) = self.cache_manager.delete(&role_cache_key).await {
            log::warn!("Failed to invalidate role cache {role_uuid}: {e}");
        }

        // Find all users with this role
        let user_repo = AdminUserRepository::new(Arc::new(self.pool.clone()));
        if let Ok(user_uuids) = user_repo.get_users_by_role(role_uuid).await {
            for user_uuid in user_uuids {
                self.invalidate_user_permissions_cache(&user_uuid).await;
            }
        }

        // Find all API keys with this role
        let api_key_repo = ApiKeyRepository::new(Arc::new(self.pool.clone()));
        if let Ok(api_key_uuids) = api_key_repo.get_api_keys_by_role(role_uuid).await {
            for api_key_uuid in api_key_uuids {
                self.invalidate_api_key_permissions_cache(&api_key_uuid)
                    .await;
            }
        }
    }

    /// Get a role by UUID with caching
    ///
    /// # Arguments
    /// * `uuid` - Role UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_role(&self, uuid: Uuid) -> Result<Option<Role>> {
        let cache_key = Self::cache_key(&uuid);

        // Try cache first
        if let Ok(Some(cached)) = self.cache_manager.get::<Role>(&cache_key).await {
            return Ok(Some(cached));
        }

        // Load from database
        let role = self.repository.get_by_uuid(uuid).await?;

        // Cache if found
        if let Some(ref role) = role {
            let ttl = self.cache_ttl;
            if let Err(e) = self.cache_manager.set(&cache_key, role, ttl).await {
                log::warn!("Failed to cache role {uuid}: {e}");
            }
        }

        Ok(role)
    }

    /// Get a role by name
    ///
    /// # Arguments
    /// * `name` - Role name
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_role_by_name(&self, name: &str) -> Result<Option<Role>> {
        self.repository.get_by_name(name).await
    }

    /// Create a new role
    ///
    /// # Arguments
    /// * `role` - Role to create
    /// * `created_by` - UUID of user creating the role
    ///
    /// # Errors
    /// Returns an error if database insert fails
    pub async fn create_role(&self, role: &Role, created_by: Uuid) -> Result<Uuid> {
        let uuid = self.repository.create(role, created_by).await?;

        // Cache the new role - retrieve it from DB to ensure it has the UUID set
        let cache_key = Self::cache_key(&uuid);
        let ttl = self.cache_ttl;
        if let Ok(Some(ref created_role)) = self.repository.get_by_uuid(uuid).await {
            if let Err(e) = self.cache_manager.set(&cache_key, created_role, ttl).await {
                log::warn!("Failed to cache new role {uuid}: {e}");
            }
        }

        Ok(uuid)
    }

    /// Update an existing role
    ///
    /// # Arguments
    /// * `role` - Role to update
    /// * `updated_by` - UUID of user updating the role
    ///
    /// # Errors
    /// Returns an error if database update fails
    pub async fn update_role(&self, role: &Role, updated_by: Uuid) -> Result<()> {
        self.repository.update(role, updated_by).await?;

        // Invalidate all caches for this role and all users/API keys that reference it
        self.invalidate_all_caches_for_role(role.base.uuid).await;

        Ok(())
    }

    /// Delete a role
    ///
    /// # Arguments
    /// * `uuid` - Role UUID
    ///
    /// # Errors
    /// Returns an error if database delete fails
    pub async fn delete_role(&self, uuid: Uuid) -> Result<()> {
        // Invalidate all caches before deleting (so reverse lookups still work)
        self.invalidate_all_caches_for_role(uuid).await;

        self.repository.delete(uuid).await?;

        Ok(())
    }

    /// List roles with pagination and sorting
    ///
    /// # Arguments
    /// * `limit` - Maximum number of roles to return (-1 for unlimited)
    /// * `offset` - Number of roles to skip
    /// * `sort_by` - Optional field to sort by
    /// * `sort_order` - Sort order (ASC or DESC), defaults to ASC
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn list_roles(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<Role>> {
        self.repository
            .list_all(limit, offset, sort_by, sort_order)
            .await
    }

    /// List roles with query validation
    ///
    /// This method validates the query parameters and returns validated parameters along with roles.
    ///
    /// # Arguments
    /// * `params` - The query parameters
    /// * `field_validator` - The `FieldValidator` instance (required for validation)
    ///
    /// # Returns
    /// A tuple of (roles, `validated_query`) where `validated_query` contains pagination metadata
    ///
    /// # Errors
    /// Returns an error if validation fails or database query fails
    pub async fn list_roles_with_query(
        &self,
        params: &crate::query_validation::ListQueryParams,
        field_validator: &FieldValidator,
    ) -> Result<(Vec<Role>, ValidatedListQuery)> {
        let validated = validate_list_query(params, "roles", field_validator, 20, 100, true, &[])
            .await
            .map_err(r_data_core_core::error::Error::Validation)?;

        let roles = self
            .repository
            .list_all(
                validated.limit,
                validated.offset,
                validated.sort_by.clone(),
                validated.sort_order.clone(),
            )
            .await?;

        Ok((roles, validated))
    }

    /// Count all roles
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn count_roles(&self) -> Result<i64> {
        self.repository.count_all().await
    }

    /// Internal helper to get roles with caching
    ///
    /// This consolidates the common logic for fetching roles by UUIDs with caching.
    async fn get_roles_with_caching(
        &self,
        cache_key: &str,
        role_uuids: Vec<Uuid>,
        entity_id: Uuid,
        entity_type: &str,
    ) -> Result<Vec<Role>> {
        // Try cache first
        if let Ok(Some(cached)) = self.cache_manager.get::<UserRoles>(cache_key).await {
            let mut roles = Vec::new();
            for uuid in &cached.role_uuids {
                if let Some(role) = self.get_role(*uuid).await? {
                    roles.push(role);
                }
            }
            return Ok(roles);
        }

        // Cache the UUIDs
        let ttl = self.cache_ttl;
        let cached = UserRoles {
            role_uuids: role_uuids.clone(),
        };
        if let Err(e) = self.cache_manager.set(cache_key, &cached, ttl).await {
            log::warn!("Failed to cache {entity_type} roles {entity_id}: {e}");
        }

        // Load full roles
        let mut roles = Vec::new();
        for uuid in role_uuids {
            if let Some(role) = self.get_role(uuid).await? {
                roles.push(role);
            }
        }

        Ok(roles)
    }

    /// Get all roles for a user (with caching)
    ///
    /// # Arguments
    /// * `user_uuid` - User UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_roles_for_user(
        &self,
        user_uuid: Uuid,
        admin_user_repo: &AdminUserRepository,
    ) -> Result<Vec<Role>> {
        let cache_key = Self::user_roles_cache_key(&user_uuid);

        // Load role UUIDs from database
        let role_uuids = admin_user_repo.get_user_roles(user_uuid).await?;

        self.get_roles_with_caching(&cache_key, role_uuids, user_uuid, "user")
            .await
    }

    /// Get all roles for an API key (with caching)
    ///
    /// # Arguments
    /// * `api_key_uuid` - API key UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_roles_for_api_key(
        &self,
        api_key_uuid: Uuid,
        api_key_repo: &ApiKeyRepository,
    ) -> Result<Vec<Role>> {
        let cache_key = Self::api_key_roles_cache_key(&api_key_uuid);

        // Load role UUIDs from database
        let role_uuids = api_key_repo.get_api_key_roles(api_key_uuid).await?;

        self.get_roles_with_caching(&cache_key, role_uuids, api_key_uuid, "API key")
            .await
    }

    /// Merge permissions from multiple roles
    ///
    /// This combines all permissions from all roles,
    /// deduplicating identical permissions.
    ///
    /// # Arguments
    /// * `roles` - Vector of roles
    ///
    /// # Returns
    /// Vector of merged permissions (deduplicated)
    #[must_use]
    pub fn merge_permissions_from_roles(roles: &[Role]) -> Vec<Permission> {
        let mut permission_set = HashSet::new();
        let mut merged_permissions = Vec::new();

        for role in roles {
            // If role is super_admin, skip (handled at permission check level)
            if role.super_admin {
                continue;
            }

            for permission in &role.permissions {
                // Use a combination of fields as the key for deduplication
                let key = (
                    permission.resource_type.clone(),
                    permission.permission_type.clone(),
                    permission.access_level.clone(),
                    permission.resource_uuids.clone(),
                    permission.constraints.clone(),
                );

                if permission_set.insert(key) {
                    merged_permissions.push(permission.clone());
                }
            }
        }

        merged_permissions
    }
}
