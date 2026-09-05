use std::sync::Arc;

use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::Role;
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};

use super::{MergedPermissions, RoleService, UserRoles};

impl RoleService {
    /// Generate cache key for role
    pub(super) fn cache_key(uuid: &Uuid) -> String {
        format!("role:{uuid}")
    }

    /// Generate cache key for user roles
    pub(super) fn user_roles_cache_key(user_uuid: &Uuid) -> String {
        format!("user_roles:{user_uuid}")
    }

    /// Generate cache key for API key roles
    pub(super) fn api_key_roles_cache_key(api_key_uuid: &Uuid) -> String {
        format!("api_key_roles:{api_key_uuid}")
    }

    /// Generate cache key for merged user permissions
    pub(super) fn user_permissions_cache_key(user_uuid: &Uuid) -> String {
        format!("user_permissions:{user_uuid}")
    }

    /// Generate cache key for merged API key permissions
    pub(super) fn api_key_permissions_cache_key(api_key_uuid: &Uuid) -> String {
        format!("api_key_permissions:{api_key_uuid}")
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
    pub(super) async fn invalidate_all_caches_for_role(&self, role_uuid: Uuid) {
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

    /// Internal helper to get roles with caching
    ///
    /// This consolidates the common logic for fetching roles by UUIDs with caching.
    pub(super) async fn get_roles_with_caching(
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

    /// Cache a `MergedPermissions` value under the given key.
    pub(super) async fn cache_merged_permissions(
        &self,
        cache_key: &str,
        merged_permissions: Vec<String>,
    ) -> Vec<String> {
        let ttl = self.cache_ttl;
        let cached = MergedPermissions {
            permissions: merged_permissions.clone(),
        };
        if let Err(e) = self.cache_manager.set(cache_key, &cached, ttl).await {
            log::warn!("Failed to cache merged permissions [{cache_key}]: {e}");
        }
        merged_permissions
    }
}
