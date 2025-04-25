// Basic test setup for API key routes
// More comprehensive tests to be implemented

#[cfg(test)]
mod tests {
    use r_data_core::{
        entity::admin_user::{ApiKeyRepository, ApiKeyRepositoryTrait},
        error::Result,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_api_key_last_used_update() -> Result<()> {
        // Setup test database
        let pool = crate::common::setup_test_db().await;
        pool.begin().await?;

        // Create a repository to work with API keys directly
        let repo = ApiKeyRepository::new(Arc::new(pool.clone()));

        // Create a test admin user
        let user_uuid = crate::common::create_test_admin_user(&pool).await?;

        // Create a test API key
        let (key_uuid, key_value) = repo
            .create_new_api_key("TestKey", "Test key for JWT test", user_uuid, 30)
            .await?;

        // Get the API key to check its initial state
        let initial_key = repo.get_by_uuid(key_uuid).await?.unwrap();
        assert!(
            initial_key.last_used_at.is_none(),
            "New key should have no last_used_at timestamp"
        );

        // Simulate authentication using the API key
        let auth_result = repo.find_api_key_for_auth(&key_value).await?;
        assert!(
            auth_result.is_some(),
            "API key authentication should succeed"
        );

        // Fetch the API key again to check if last_used_at was updated
        let updated_key = repo.get_by_uuid(key_uuid).await?.unwrap();
        assert!(
            updated_key.last_used_at.is_some(),
            "API key's last_used_at should be updated after auth"
        );

        Ok(())
    }
}
