#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::permission_scheme::{Permission, PermissionScheme};
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository, PermissionSchemeRepository};
use std::collections::HashSet;

/// Service for managing permission schemes with caching
pub struct PermissionSchemeService {
    repository: PermissionSchemeRepository,
    cache_manager: Arc<CacheManager>,
    cache_ttl: Option<u64>,
}

impl PermissionSchemeService {
    /// Create a new permission scheme service
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `cache_manager` - Cache manager for caching schemes
    /// * `cache_ttl` - Optional cache TTL in seconds (uses `entity_definition_ttl` if None)
    #[must_use]
    pub const fn new(
        pool: PgPool,
        cache_manager: Arc<CacheManager>,
        cache_ttl: Option<u64>,
    ) -> Self {
        Self {
            repository: PermissionSchemeRepository::new(pool),
            cache_manager,
            cache_ttl,
        }
    }

    /// Generate cache key for permission scheme
    fn cache_key(&self, uuid: &Uuid) -> String {
        format!("permission_scheme:{uuid}")
    }

    /// Generate cache key for user permission schemes
    fn user_schemes_cache_key(&self, user_uuid: &Uuid) -> String {
        format!("user_permission_schemes:{user_uuid}")
    }

    /// Generate cache key for API key permission schemes
    fn api_key_schemes_cache_key(&self, api_key_uuid: &Uuid) -> String {
        format!("api_key_permission_schemes:{api_key_uuid}")
    }

    /// Generate cache key for merged user permissions
    fn user_permissions_cache_key(&self, user_uuid: &Uuid, role: &str) -> String {
        format!("user_permissions:{user_uuid}:{role}")
    }

    /// Generate cache key for merged API key permissions
    fn api_key_permissions_cache_key(&self, api_key_uuid: &Uuid) -> String {
        format!("api_key_permissions:{api_key_uuid}")
    }

    /// Get merged permissions for a user and role with caching
    ///
    /// # Arguments
    /// * `user_uuid` - User UUID
    /// * `role` - User role
    /// * `admin_user_repo` - Admin user repository
    ///
    /// # Returns
    /// Vector of merged permission strings
    pub async fn get_merged_permissions_for_user(
        &self,
        user_uuid: Uuid,
        role: &str,
        admin_user_repo: &AdminUserRepository,
    ) -> Result<Vec<String>> {
        let cache_key = self.user_permissions_cache_key(&user_uuid, role);

        // Try cache first
        if let Ok(Some(cached)) = self.cache_manager.get::<Vec<String>>(&cache_key).await {
            return Ok(cached);
        }

        // Load schemes and merge permissions
        let schemes = self
            .get_schemes_for_user(user_uuid, admin_user_repo)
            .await?;

        // Merge permissions from all schemes for the role
        let mut permission_set = HashSet::new();
        let mut merged_permissions = Vec::new();

        for scheme in &schemes {
            let scheme_permissions = scheme.get_permissions_as_strings(role);
            for perm in scheme_permissions {
                if permission_set.insert(perm.clone()) {
                    merged_permissions.push(perm);
                }
            }
        }

        // Cache the merged permissions
        let ttl = self.cache_ttl;
        if let Err(e) = self
            .cache_manager
            .set(&cache_key, &merged_permissions, ttl)
            .await
        {
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
    /// Vector of merged permission strings (from all roles in schemes)
    pub async fn get_merged_permissions_for_api_key(
        &self,
        api_key_uuid: Uuid,
        api_key_repo: &ApiKeyRepository,
    ) -> Result<Vec<String>> {
        let cache_key = self.api_key_permissions_cache_key(&api_key_uuid);

        // Try cache first
        if let Ok(Some(cached)) = self.cache_manager.get::<Vec<String>>(&cache_key).await {
            return Ok(cached);
        }

        // Load schemes and merge permissions from all roles
        let schemes = self
            .get_schemes_for_api_key(api_key_uuid, api_key_repo)
            .await?;
        let mut permission_set = HashSet::new();
        let mut merged_permissions = Vec::new();

        for scheme in &schemes {
            // API keys don't have roles, so merge permissions from all roles
            for (_role, permissions) in &scheme.role_permissions {
                for permission in permissions {
                    let perm_str = format!("{}", permission.permission_type).to_lowercase();
                    let perm_string = if let Some(path) =
                        permission.constraints.as_ref().and_then(|c| c.get("path"))
                    {
                        format!(
                            "{}:{}:{}",
                            permission.resource_type.as_str(),
                            path,
                            perm_str
                        )
                    } else {
                        format!("{}:{}", permission.resource_type.as_str(), perm_str)
                    };

                    if permission_set.insert(perm_string.clone()) {
                        merged_permissions.push(perm_string);
                    }
                }
            }
        }

        // Cache the merged permissions
        let ttl = self.cache_ttl;
        if let Err(e) = self
            .cache_manager
            .set(&cache_key, &merged_permissions, ttl)
            .await
        {
            log::warn!("Failed to cache merged permissions for API key {api_key_uuid}: {e}");
        }

        Ok(merged_permissions)
    }

    /// Invalidate cached permissions for a user (all roles)
    ///
    /// This invalidates both the schemes cache and any cached merged permissions.
    /// Since we don't know all roles, we invalidate the schemes cache which will
    /// cause permissions to be recalculated on next access.
    ///
    /// # Arguments
    /// * `user_uuid` - User UUID
    pub async fn invalidate_user_permissions_cache(&self, user_uuid: &Uuid) {
        // Invalidate schemes cache - this will cause permissions to be recalculated
        let cache_key = self.user_schemes_cache_key(user_uuid);
        if let Err(e) = self.cache_manager.delete(&cache_key).await {
            log::warn!("Failed to invalidate user schemes cache {user_uuid}: {e}");
        }
        // Note: We can't invalidate merged permissions cache for all roles since we don't know them.
        // The merged permissions will be recalculated when get_merged_permissions_for_user is called
        // because it will reload schemes (cache miss) and recalculate.
    }

    /// Invalidate cached permissions for an API key
    ///
    /// # Arguments
    /// * `api_key_uuid` - API key UUID
    pub async fn invalidate_api_key_permissions_cache(&self, api_key_uuid: &Uuid) {
        // Invalidate merged permissions cache
        let cache_key = self.api_key_permissions_cache_key(api_key_uuid);
        if let Err(e) = self.cache_manager.delete(&cache_key).await {
            log::warn!("Failed to invalidate API key permissions cache {api_key_uuid}: {e}");
        }
        // Also invalidate schemes cache
        let schemes_cache_key = self.api_key_schemes_cache_key(api_key_uuid);
        if let Err(e) = self.cache_manager.delete(&schemes_cache_key).await {
            log::warn!("Failed to invalidate API key schemes cache {api_key_uuid}: {e}");
        }
    }

    /// Get a permission scheme by UUID with caching
    ///
    /// # Arguments
    /// * `uuid` - Scheme UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_scheme(&self, uuid: Uuid) -> Result<Option<PermissionScheme>> {
        let cache_key = self.cache_key(&uuid);

        // Try cache first
        if let Ok(Some(cached)) = self.cache_manager.get::<PermissionScheme>(&cache_key).await {
            return Ok(Some(cached));
        }

        // Load from database
        let scheme = self.repository.get_by_uuid(uuid).await?;

        // Cache if found
        if let Some(ref scheme) = scheme {
            let ttl = self.cache_ttl;
            if let Err(e) = self.cache_manager.set(&cache_key, scheme, ttl).await {
                log::warn!("Failed to cache permission scheme {uuid}: {e}");
            }
        }

        Ok(scheme)
    }

    /// Get a permission scheme by name
    ///
    /// # Arguments
    /// * `name` - Scheme name
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_scheme_by_name(&self, name: &str) -> Result<Option<PermissionScheme>> {
        self.repository.get_by_name(name).await
    }

    /// Get permission scheme for a user
    ///
    /// If user has no scheme assigned, returns None (empty permissions).
    /// `SuperAdmin` always has all permissions (handled at application level).
    ///
    /// # Arguments
    /// * `scheme_uuid` - Optional scheme UUID from user
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_scheme_for_user(
        &self,
        scheme_uuid: Option<Uuid>,
    ) -> Result<Option<PermissionScheme>> {
        match scheme_uuid {
            Some(uuid) => self.get_scheme(uuid).await,
            None => Ok(None),
        }
    }

    /// Create a new permission scheme
    ///
    /// # Arguments
    /// * `scheme` - Permission scheme to create
    /// * `created_by` - UUID of user creating the scheme
    ///
    /// # Errors
    /// Returns an error if database insert fails
    pub async fn create_scheme(&self, scheme: &PermissionScheme, created_by: Uuid) -> Result<Uuid> {
        let uuid = self.repository.create(scheme, created_by).await?;

        // Cache the new scheme
        let cache_key = self.cache_key(&uuid);
        let ttl = self.cache_ttl;
        if let Err(e) = self.cache_manager.set(&cache_key, scheme, ttl).await {
            log::warn!("Failed to cache new permission scheme {uuid}: {e}");
        }

        Ok(uuid)
    }

    /// Update an existing permission scheme
    ///
    /// # Arguments
    /// * `scheme` - Permission scheme to update
    /// * `updated_by` - UUID of user updating the scheme
    ///
    /// # Errors
    /// Returns an error if database update fails
    pub async fn update_scheme(&self, scheme: &PermissionScheme, updated_by: Uuid) -> Result<()> {
        self.repository.update(scheme, updated_by).await?;

        // Invalidate scheme cache
        let cache_key = self.cache_key(&scheme.base.uuid);
        if let Err(e) = self.cache_manager.delete(&cache_key).await {
            log::warn!(
                "Failed to invalidate cache for permission scheme {}: {}",
                scheme.base.uuid,
                e
            );
        }

        // Note: User/API key permission caches will be invalidated when schemes are reassigned
        // or when they're accessed next (cache miss will trigger reload)

        Ok(())
    }

    /// Delete a permission scheme
    ///
    /// # Arguments
    /// * `uuid` - Scheme UUID
    ///
    /// # Errors
    /// Returns an error if database delete fails
    pub async fn delete_scheme(&self, uuid: Uuid) -> Result<()> {
        self.repository.delete(uuid).await?;

        // Invalidate cache
        let cache_key = self.cache_key(&uuid);
        if let Err(e) = self.cache_manager.delete(&cache_key).await {
            log::warn!("Failed to invalidate cache for permission scheme {uuid}: {e}");
        }

        Ok(())
    }

    /// List permission schemes with pagination
    ///
    /// # Arguments
    /// * `limit` - Maximum number of schemes to return
    /// * `offset` - Number of schemes to skip
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn list_schemes(&self, limit: i64, offset: i64) -> Result<Vec<PermissionScheme>> {
        self.repository.list_all(limit, offset).await
    }

    /// Count all permission schemes
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn count_schemes(&self) -> Result<i64> {
        self.repository.count_all().await
    }

    /// Get all permission schemes for a user (with caching)
    ///
    /// # Arguments
    /// * `user_uuid` - User UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_schemes_for_user(
        &self,
        user_uuid: Uuid,
        admin_user_repo: &AdminUserRepository,
    ) -> Result<Vec<PermissionScheme>> {
        let cache_key = self.user_schemes_cache_key(&user_uuid);

        // Try cache first
        if let Ok(Some(cached)) = self.cache_manager.get::<Vec<Uuid>>(&cache_key).await {
            // Load schemes from cached UUIDs
            let mut schemes = Vec::new();
            for uuid in cached {
                if let Some(scheme) = self.get_scheme(uuid).await? {
                    schemes.push(scheme);
                }
            }
            return Ok(schemes);
        }

        // Load scheme UUIDs from database
        let scheme_uuids = admin_user_repo
            .get_user_permission_schemes(user_uuid)
            .await?;

        // Cache the UUIDs
        let ttl = self.cache_ttl;
        if let Err(e) = self.cache_manager.set(&cache_key, &scheme_uuids, ttl).await {
            log::warn!("Failed to cache user permission schemes {user_uuid}: {e}");
        }

        // Load full schemes
        let mut schemes = Vec::new();
        for uuid in scheme_uuids {
            if let Some(scheme) = self.get_scheme(uuid).await? {
                schemes.push(scheme);
            }
        }

        Ok(schemes)
    }

    /// Get all permission schemes for an API key (with caching)
    ///
    /// # Arguments
    /// * `api_key_uuid` - API key UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_schemes_for_api_key(
        &self,
        api_key_uuid: Uuid,
        api_key_repo: &ApiKeyRepository,
    ) -> Result<Vec<PermissionScheme>> {
        let cache_key = self.api_key_schemes_cache_key(&api_key_uuid);

        // Try cache first
        if let Ok(Some(cached)) = self.cache_manager.get::<Vec<Uuid>>(&cache_key).await {
            // Load schemes from cached UUIDs
            let mut schemes = Vec::new();
            for uuid in cached {
                if let Some(scheme) = self.get_scheme(uuid).await? {
                    schemes.push(scheme);
                }
            }
            return Ok(schemes);
        }

        // Load scheme UUIDs from database
        let scheme_uuids = api_key_repo
            .get_api_key_permission_schemes(api_key_uuid)
            .await?;

        // Cache the UUIDs
        let ttl = self.cache_ttl;
        if let Err(e) = self.cache_manager.set(&cache_key, &scheme_uuids, ttl).await {
            log::warn!("Failed to cache API key permission schemes {api_key_uuid}: {e}");
        }

        // Load full schemes
        let mut schemes = Vec::new();
        for uuid in scheme_uuids {
            if let Some(scheme) = self.get_scheme(uuid).await? {
                schemes.push(scheme);
            }
        }

        Ok(schemes)
    }

    /// Merge permissions from multiple schemes for a role
    ///
    /// This combines all permissions from all schemes for the given role,
    /// deduplicating identical permissions.
    ///
    /// # Arguments
    /// * `schemes` - Vector of permission schemes
    /// * `role` - Role name to get permissions for
    ///
    /// # Returns
    /// Vector of merged permissions (deduplicated)
    #[must_use]
    pub fn merge_permissions_from_schemes(
        schemes: &[PermissionScheme],
        role: &str,
    ) -> Vec<Permission> {
        let mut permission_set = HashSet::new();
        let mut merged_permissions = Vec::new();

        for scheme in schemes {
            if let Some(permissions) = scheme.get_role_permissions(role) {
                for permission in permissions {
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
        }

        merged_permissions
    }
}
