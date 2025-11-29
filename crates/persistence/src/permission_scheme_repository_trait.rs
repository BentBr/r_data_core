#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use uuid::Uuid;

use crate::core::error::Result;
use crate::core::permissions::permission_scheme::PermissionScheme;

/// Trait for permission scheme repository operations
#[async_trait]
pub trait PermissionSchemeRepositoryTrait: Send + Sync {
    /// Get a permission scheme by UUID
    ///
    /// # Arguments
    /// * `uuid` - Scheme UUID
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<PermissionScheme>>;

    /// Get a permission scheme by name
    ///
    /// # Arguments
    /// * `name` - Scheme name
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_by_name(&self, name: &str) -> Result<Option<PermissionScheme>>;

    /// Create a new permission scheme
    ///
    /// # Arguments
    /// * `scheme` - Permission scheme to create
    /// * `created_by` - UUID of user creating the scheme
    ///
    /// # Errors
    /// Returns an error if database insert fails
    async fn create(&self, scheme: &PermissionScheme, created_by: Uuid) -> Result<Uuid>;

    /// Update an existing permission scheme
    ///
    /// # Arguments
    /// * `scheme` - Permission scheme to update
    /// * `updated_by` - UUID of user updating the scheme
    ///
    /// # Errors
    /// Returns an error if database update fails
    async fn update(&self, scheme: &PermissionScheme, updated_by: Uuid) -> Result<()>;

    /// Delete a permission scheme
    ///
    /// # Arguments
    /// * `uuid` - Scheme UUID
    ///
    /// # Errors
    /// Returns an error if database delete fails
    async fn delete(&self, uuid: Uuid) -> Result<()>;
}
