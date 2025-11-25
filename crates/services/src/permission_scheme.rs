#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use r_data_core_core::cache::CacheManager;
use r_data_core_core::error::Result;
use r_data_core_core::permissions::permission_scheme::PermissionScheme;
use r_data_core_persistence::PermissionSchemeRepository;

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
    pub async fn create_scheme(
        &self,
        scheme: &PermissionScheme,
        created_by: Uuid,
    ) -> Result<Uuid> {
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
    pub async fn update_scheme(
        &self,
        scheme: &PermissionScheme,
        updated_by: Uuid,
    ) -> Result<()> {
        self.repository.update(scheme, updated_by).await?;

        // Invalidate cache
        let cache_key = self.cache_key(&scheme.base.uuid);
        if let Err(e) = self.cache_manager.delete(&cache_key).await {
            log::warn!(
                "Failed to invalidate cache for permission scheme {}: {}",
                scheme.base.uuid,
                e
            );
        }

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
}

