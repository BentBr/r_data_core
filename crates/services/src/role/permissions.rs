use std::collections::HashSet;

use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::{Permission, Role};
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};

use super::{MergedPermissions, RoleService};

impl RoleService {
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

        Ok(self
            .cache_merged_permissions(&cache_key, merged_permissions)
            .await)
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

        Ok(self
            .cache_merged_permissions(&cache_key, merged_permissions)
            .await)
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
