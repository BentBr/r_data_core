use r_data_core_core::admin_user::ApiKey;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::error::Result;
use r_data_core_persistence::{ApiKeyRepository, ApiKeyRepositoryTrait};
use std::sync::Arc;
use uuid::Uuid;

use crate::query_validation::{validate_list_query, FieldValidator, ValidatedListQuery};

/// Service for handling API key operations
pub struct ApiKeyService {
    repository: Arc<dyn ApiKeyRepositoryTrait>,
    cache_manager: Option<Arc<CacheManager>>,
    api_key_ttl: u64,
}

impl ApiKeyService {
    /// Create a new API key service with a concrete repository
    pub fn new(repository: Arc<dyn ApiKeyRepositoryTrait>) -> Self {
        Self {
            repository,
            cache_manager: None,
            api_key_ttl: 600, // Default 10 minutes
        }
    }

    /// Create a new API key service with cache manager
    pub fn with_cache(
        repository: Arc<dyn ApiKeyRepositoryTrait>,
        cache_manager: Arc<CacheManager>,
        api_key_ttl: u64,
    ) -> Self {
        Self {
            repository,
            cache_manager: Some(cache_manager),
            api_key_ttl,
        }
    }

    /// Create a new API key service from a concrete repository
    #[must_use]
    pub fn from_repository(repository: ApiKeyRepository) -> Self {
        Self {
            repository: Arc::new(repository),
            cache_manager: None,
            api_key_ttl: 600, // Default 10 minutes
        }
    }

    /// Generate cache key for API key by hash
    fn cache_key_by_hash(key_hash: &str) -> String {
        format!("api_key:hash:{key_hash}")
    }

    /// Create a new API key
    ///
    /// # Errors
    /// Returns an error if validation fails or database operation fails
    pub async fn create_api_key(
        &self,
        name: &str,
        description: &str,
        created_by: Uuid,
        expires_in_days: i32,
    ) -> Result<(Uuid, String)> {
        // Validation
        if name.is_empty() {
            return Err(r_data_core_core::error::Error::Validation(
                "API key name cannot be empty".to_string(),
            ));
        }

        if expires_in_days < 0 {
            return Err(r_data_core_core::error::Error::Validation(
                "Expiration days cannot be negative".to_string(),
            ));
        }

        self.repository
            .create_new_api_key(name, description, created_by, expires_in_days)
            .await
    }

    /// Validate an API key and return user information if valid
    ///
    /// # Errors
    /// Returns an error if database operation fails
    pub async fn validate_api_key(&self, api_key: &str) -> Result<Option<(ApiKey, Uuid)>> {
        if api_key.is_empty() {
            return Ok(None);
        }

        // Hash the API key for cache lookup
        let key_hash = match ApiKey::hash_api_key(api_key) {
            Ok(hash) => hash,
            Err(e) => {
                log::warn!("Failed to hash API key for cache: {e}");
                // Fall back to repository lookup
                return self.repository.find_api_key_for_auth(api_key).await;
            }
        };

        // Check cache first if cache manager is available
        if let Some(cache) = &self.cache_manager {
            let cache_key = Self::cache_key_by_hash(&key_hash);
            if let Ok(Some(cached)) = cache.get::<(ApiKey, Uuid)>(&cache_key).await {
                // Cache hit - return cached result (skip last_used_at update for performance)
                return Ok(Some(cached));
            }
        }

        // Cache miss - query repository
        let result = self.repository.find_api_key_for_auth(api_key).await?;

        // Cache the result if valid and cache manager is available
        if let Some((ref key, ref user_uuid)) = result {
            if let Some(cache) = &self.cache_manager {
                let cache_key = Self::cache_key_by_hash(&key_hash);
                // Use configured TTL (0 = no expiration, but we use Some() to respect TTL)
                let ttl = if self.api_key_ttl > 0 {
                    Some(self.api_key_ttl)
                } else {
                    None // No expiration
                };
                if let Err(e) = cache.set(&cache_key, &(key.clone(), *user_uuid), ttl).await {
                    log::warn!("Failed to cache API key validation result: {e}");
                }
            }
        }

        Ok(result)
    }

    /// List all API keys for a user
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_keys_for_user(
        &self,
        user_uuid: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ApiKey>> {
        // Validate input
        let limit = if limit <= 0 { 50 } else { limit };
        let offset = if offset < 0 { 0 } else { offset };

        self.repository
            .list_by_user(user_uuid, limit, offset, None, None)
            .await
    }

    /// List API keys for a user with query validation
    ///
    /// This method validates the query parameters and returns validated parameters along with API keys.
    ///
    /// # Arguments
    /// * `user_uuid` - The user UUID
    /// * `params` - The query parameters
    /// * `field_validator` - The `FieldValidator` instance (required for validation)
    ///
    /// # Returns
    /// A tuple of (`api_keys`, `validated_query`) where `validated_query` contains pagination metadata
    ///
    /// # Errors
    /// Returns an error if validation fails or database query fails
    pub async fn list_keys_for_user_with_query(
        &self,
        user_uuid: Uuid,
        params: &crate::query_validation::ListQueryParams,
        field_validator: &FieldValidator,
    ) -> Result<(Vec<ApiKey>, ValidatedListQuery)> {
        let validated =
            validate_list_query(params, "api_keys", field_validator, 20, 100, true, &[])
                .await
                .map_err(r_data_core_core::error::Error::Validation)?;

        let keys = self
            .repository
            .list_by_user(
                user_uuid,
                validated.limit,
                validated.offset,
                validated.sort_by.clone(),
                validated.sort_order.clone(),
            )
            .await?;

        Ok((keys, validated))
    }

    /// Revoke an API key
    ///
    /// # Errors
    /// Returns an error if the key is not found, user doesn't have permission, or database operation fails
    pub async fn revoke_key(&self, key_uuid: Uuid, user_uuid: Uuid) -> Result<()> {
        // Verify ownership first
        let key = self.repository.get_by_uuid(key_uuid).await?;

        match key {
            Some(key) if key.user_uuid == user_uuid => {
                // Revoke the key
                let result = self.repository.revoke(key_uuid).await;

                // Invalidate cache if revocation succeeded
                if result.is_ok() {
                    if let Some(cache) = &self.cache_manager {
                        let cache_key = Self::cache_key_by_hash(&key.key_hash);
                        if let Err(e) = cache.delete(&cache_key).await {
                            log::warn!("Failed to invalidate API key cache: {e}");
                        }
                    }
                }

                result
            }
            Some(_) => Err(r_data_core_core::error::Error::Forbidden(
                "You don't have permission to revoke this API key".to_string(),
            )),
            None => Err(r_data_core_core::error::Error::NotFound(
                "API key not found".to_string(),
            )),
        }
    }

    /// Get a key by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_key(&self, key_uuid: Uuid) -> Result<Option<ApiKey>> {
        self.repository.get_by_uuid(key_uuid).await
    }

    /// Reassign an API key to a different user
    ///
    /// # Errors
    /// Returns an error if the key is not found or database operation fails
    pub async fn reassign_key(&self, key_uuid: Uuid, new_user_uuid: Uuid) -> Result<()> {
        // Verify the key exists
        let key = self.get_key(key_uuid).await?;
        if key.is_none() {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "API key with UUID {key_uuid} not found"
            )));
        }

        // Reassign the key
        self.repository.reassign(key_uuid, new_user_uuid).await
    }
}

#[cfg(test)]
mod tests;
