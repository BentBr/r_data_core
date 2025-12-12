#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use uuid::Uuid;

use crate::core::error::Result;
use crate::core::permissions::role::Role;

/// Trait for role repository operations
#[async_trait]
pub trait RoleRepositoryTrait: Send + Sync {
    /// Get a role by UUID
    ///
    /// # Arguments
    /// * `uuid` - Role UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<Role>>;

    /// Get a role by name
    ///
    /// # Arguments
    /// * `name` - Role name
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_by_name(&self, name: &str) -> Result<Option<Role>>;

    /// Create a new role
    ///
    /// # Arguments
    /// * `role` - Role to create
    /// * `created_by` - UUID of user creating the role
    ///
    /// # Errors
    /// Returns an error if database insert fails
    async fn create(&self, role: &Role, created_by: Uuid) -> Result<Uuid>;

    /// Update an existing role
    ///
    /// # Arguments
    /// * `role` - Role to update
    /// * `updated_by` - UUID of user updating the role
    ///
    /// # Errors
    /// Returns an error if database update fails
    async fn update(&self, role: &Role, updated_by: Uuid) -> Result<()>;

    /// Delete a role
    ///
    /// # Arguments
    /// * `uuid` - Role UUID
    ///
    /// # Errors
    /// Returns an error if database delete fails
    async fn delete(&self, uuid: Uuid) -> Result<()>;

    /// List all roles with pagination and sorting
    ///
    /// # Arguments
    /// * `limit` - Maximum number of roles to return (-1 for unlimited)
    /// * `offset` - Number of roles to skip
    /// * `sort_by` - Optional field to sort by
    /// * `sort_order` - Sort order (ASC or DESC), defaults to ASC
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn list_all(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<Role>>;

    /// Count all roles
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn count_all(&self) -> Result<i64>;
}
