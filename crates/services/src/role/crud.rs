use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::permissions::role::Role;
use r_data_core_core::system_log::SystemLogResourceType;
use r_data_core_persistence::{AdminUserRepository, ApiKeyRepository};

use crate::query_validation::{validate_list_query, FieldValidator, ValidatedListQuery};

use super::RoleService;

impl RoleService {
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

        if let Some(ref log) = self.system_log {
            log.log_entity_created(
                Some(created_by),
                SystemLogResourceType::Role,
                uuid,
                &format!("Role '{}' created", role.name),
                Some(serde_json::json!({"name": role.name})),
            )
            .await;
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

        if let Some(ref log) = self.system_log {
            log.log_entity_updated(
                Some(updated_by),
                SystemLogResourceType::Role,
                role.base.uuid,
                &format!("Role '{}' updated", role.name),
                Some(serde_json::json!({"name": role.name})),
            )
            .await;
        }

        Ok(())
    }

    /// Delete a role
    ///
    /// # Arguments
    /// * `uuid` - Role UUID
    /// * `actor_uuid` - UUID of user performing the deletion
    ///
    /// # Errors
    /// Returns an error if database delete fails
    pub async fn delete_role(&self, uuid: Uuid, actor_uuid: Uuid) -> Result<()> {
        // Capture the role name before deletion for audit log
        let role_name = self
            .repository
            .get_by_uuid(uuid)
            .await
            .ok()
            .flatten()
            .map_or_else(|| uuid.to_string(), |r| r.name);

        // Invalidate all caches before deleting (so reverse lookups still work)
        self.invalidate_all_caches_for_role(uuid).await;

        self.repository.delete(uuid).await?;

        if let Some(ref log) = self.system_log {
            log.log_entity_deleted(
                Some(actor_uuid),
                SystemLogResourceType::Role,
                uuid,
                &format!("Role '{role_name}' deleted"),
                Some(serde_json::json!({"name": role_name})),
            )
            .await;
        }

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
}
