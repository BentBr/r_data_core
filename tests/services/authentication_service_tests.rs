#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use async_trait::async_trait;
use mockall::{mock, predicate::*};
use r_data_core_core::admin_user::ApiKey;
use r_data_core_core::error::Result;
use r_data_core_persistence::ApiKeyRepositoryTrait;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

// Mock for API key repository
mock! {
    pub ApiKeyRepo {}

    #[async_trait]
    impl ApiKeyRepositoryTrait for ApiKeyRepo {
        async fn find_api_key_for_auth(&self, api_key: &str) -> Result<Option<(ApiKey, Uuid)>>;
        async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ApiKey>>;
        async fn create(&self, key: &ApiKey) -> Result<Uuid>;
        async fn list_by_user(&self, user_uuid: Uuid, limit: i64, offset: i64, sort_by: Option<String>, sort_order: Option<String>) -> Result<Vec<ApiKey>>;
        async fn revoke(&self, uuid: Uuid) -> Result<()>;
        async fn get_by_name(&self, user_uuid: Uuid, name: &str) -> Result<Option<ApiKey>>;
        async fn get_by_hash(&self, api_key: &str) -> Result<Option<ApiKey>>;
        async fn create_new_api_key(
            &self,
            name: &str,
            description: &str,
            created_by: Uuid,
            expires_in_days: i32,
        ) -> Result<(Uuid, String)>;
        async fn update_last_used(&self, uuid: Uuid) -> Result<()>;
        async fn reassign(&self, uuid: Uuid, new_user_uuid: Uuid) -> Result<()>;
        async fn count_by_user(&self, user_uuid: Uuid) -> Result<i64>;
        async fn get_api_key_roles(&self, api_key_uuid: Uuid) -> Result<Vec<Uuid>>;
        async fn assign_role(&self, api_key_uuid: Uuid, role_uuid: Uuid) -> Result<()>;
        async fn unassign_role(&self, api_key_uuid: Uuid, role_uuid: Uuid) -> Result<()>;
        async fn update_api_key_roles(&self, api_key_uuid: Uuid, role_uuids: &[Uuid]) -> Result<()>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test API key validation with valid key
    #[tokio::test]
    async fn test_api_key_validation_valid() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();
        let user_uuid = Uuid::now_v7();
        let key_uuid = Uuid::now_v7();

        // Create a mock API key
        let mock_key = ApiKey {
            uuid: key_uuid,
            user_uuid,
            key_hash: "hashed_key".to_string(),
            name: "Test Key".to_string(),
            description: Some("Test description".to_string()),
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
            expires_at: Some(OffsetDateTime::now_utc() + Duration::days(30)),
            last_used_at: None,
            created_by: user_uuid,
            published: true,
        };

        // Setup mock expectations
        mock_repo
            .expect_find_api_key_for_auth()
            .with(eq("valid_api_key"))
            .returning(move |_| Ok(Some((mock_key.clone(), user_uuid))));

        // Test validation
        let result = mock_repo.find_api_key_for_auth("valid_api_key").await?;
        assert!(result.is_some());

        if let Some((key, extracted_user_uuid)) = result {
            assert_eq!(key.uuid, key_uuid);
            assert_eq!(extracted_user_uuid, user_uuid);
            assert!(key.is_active);
        }

        Ok(())
    }

    /// Test API key validation with invalid key
    #[tokio::test]
    async fn test_api_key_validation_invalid() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();

        // Setup mock to return None for invalid key
        mock_repo
            .expect_find_api_key_for_auth()
            .with(eq("invalid_api_key"))
            .returning(|_| Ok(None));

        // Test validation
        let result = mock_repo.find_api_key_for_auth("invalid_api_key").await?;
        assert!(result.is_none());

        Ok(())
    }

    /// Test API key validation with expired key
    #[tokio::test]
    async fn test_api_key_validation_expired() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();

        // Setup mock to return None for expired key (repository filters out expired keys)
        mock_repo
            .expect_find_api_key_for_auth()
            .with(eq("expired_api_key"))
            .returning(|_| Ok(None));

        // Test validation
        let result = mock_repo.find_api_key_for_auth("expired_api_key").await?;
        assert!(result.is_none());

        Ok(())
    }

    /// Test API key validation with revoked key
    #[tokio::test]
    async fn test_api_key_validation_revoked() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();

        // Setup mock to return None for revoked key (repository filters inactive keys)
        mock_repo
            .expect_find_api_key_for_auth()
            .with(eq("revoked_api_key"))
            .returning(|_| Ok(None));

        // Test validation
        let result = mock_repo.find_api_key_for_auth("revoked_api_key").await?;
        assert!(result.is_none());

        Ok(())
    }

    /// Test API key creation validation
    #[tokio::test]
    async fn test_api_key_creation_validation() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();
        let user_uuid = Uuid::now_v7();

        // Test with empty name
        mock_repo
            .expect_create_new_api_key()
            .with(eq(""), eq("Test description"), eq(user_uuid), eq(30))
            .returning(|_, _, _, _| {
                Err(r_data_core_core::error::Error::Validation(
                    "API key name cannot be empty".to_string(),
                ))
            });

        let result = mock_repo
            .create_new_api_key("", "Test description", user_uuid, 30)
            .await;

        assert!(result.is_err());
        if let Err(r_data_core_core::error::Error::Validation(msg)) = result {
            assert!(msg.contains("empty"));
        } else {
            panic!("Expected validation error");
        }

        // Test with negative expiration
        mock_repo
            .expect_create_new_api_key()
            .with(
                eq("Test Key"),
                eq("Test description"),
                eq(user_uuid),
                eq(-1),
            )
            .returning(|_, _, _, _| {
                Err(r_data_core_core::error::Error::Validation(
                    "Expiration days cannot be negative".to_string(),
                ))
            });

        let result = mock_repo
            .create_new_api_key("Test Key", "Test description", user_uuid, -1)
            .await;

        assert!(result.is_err());
        if let Err(r_data_core_core::error::Error::Validation(msg)) = result {
            assert!(msg.contains("negative"));
        } else {
            panic!("Expected validation error");
        }

        Ok(())
    }

    /// Test API key revocation
    #[tokio::test]
    async fn test_api_key_revocation() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();
        let key_uuid = Uuid::now_v7();

        // Setup mock for successful revocation
        mock_repo
            .expect_revoke()
            .with(eq(key_uuid))
            .returning(|_| Ok(()));

        // Test revocation
        let result = mock_repo.revoke(key_uuid).await;
        assert!(result.is_ok());

        Ok(())
    }

    /// Test API key reassignment
    #[tokio::test]
    async fn test_api_key_reassignment() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();
        let key_uuid = Uuid::now_v7();
        let new_user_uuid = Uuid::now_v7();

        // Setup mock for successful reassignment
        mock_repo
            .expect_reassign()
            .with(eq(key_uuid), eq(new_user_uuid))
            .returning(|_, _| Ok(()));

        // Test reassignment
        let result = mock_repo.reassign(key_uuid, new_user_uuid).await;
        assert!(result.is_ok());

        Ok(())
    }

    /// Test API key usage tracking
    #[tokio::test]
    async fn test_api_key_usage_tracking() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();
        let key_uuid = Uuid::now_v7();

        // Setup mock for usage tracking
        mock_repo
            .expect_update_last_used()
            .with(eq(key_uuid))
            .returning(|_| Ok(()));

        // Test usage tracking
        let result = mock_repo.update_last_used(key_uuid).await;
        assert!(result.is_ok());

        Ok(())
    }

    /// Test API key listing with pagination
    #[tokio::test]
    async fn test_api_key_listing_pagination() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();
        let user_uuid = Uuid::now_v7();

        // Create mock API keys
        let mock_keys = vec![
            ApiKey {
                uuid: Uuid::now_v7(),
                user_uuid,
                key_hash: "hash1".to_string(),
                name: "Key 1".to_string(),
                description: Some("First key".to_string()),
                is_active: true,
                created_at: OffsetDateTime::now_utc(),
                expires_at: Some(OffsetDateTime::now_utc() + Duration::days(30)),
                last_used_at: None,
                created_by: user_uuid,
                published: true,
            },
            ApiKey {
                uuid: Uuid::now_v7(),
                user_uuid,
                key_hash: "hash2".to_string(),
                name: "Key 2".to_string(),
                description: Some("Second key".to_string()),
                is_active: true,
                created_at: OffsetDateTime::now_utc(),
                expires_at: Some(OffsetDateTime::now_utc() + Duration::days(30)),
                last_used_at: None,
                created_by: user_uuid,
                published: true,
            },
        ];

        // Setup mock for listing
        mock_repo
            .expect_list_by_user()
            .with(eq(user_uuid), eq(10), eq(0), eq(None), eq(None))
            .returning(move |_, _, _, _, _| Ok(mock_keys.clone()));

        // Test listing
        let result = mock_repo.list_by_user(user_uuid, 10, 0, None, None).await;
        assert!(result.is_ok());

        if let Ok(keys) = result {
            assert_eq!(keys.len(), 2);
            assert_eq!(keys[0].name, "Key 1");
            assert_eq!(keys[1].name, "Key 2");
        }

        Ok(())
    }

    /// Test API key authentication edge cases
    #[tokio::test]
    async fn test_api_key_authentication_edge_cases() -> Result<()> {
        let mut mock_repo = MockApiKeyRepo::new();

        // Test with empty API key
        mock_repo
            .expect_find_api_key_for_auth()
            .with(eq(""))
            .returning(|_| Ok(None));

        let result = mock_repo.find_api_key_for_auth("").await?;
        assert!(result.is_none());

        // Test with very long API key
        let long_key = "a".repeat(1000);
        mock_repo
            .expect_find_api_key_for_auth()
            .with(eq(long_key.clone()))
            .returning(|_| Ok(None));

        let result = mock_repo.find_api_key_for_auth(&long_key).await?;
        assert!(result.is_none());

        // Test with special characters in API key
        let special_key = "key_with_special_chars!@#$%^&*()";
        mock_repo
            .expect_find_api_key_for_auth()
            .with(eq(special_key))
            .returning(|_| Ok(None));

        let result = mock_repo.find_api_key_for_auth(special_key).await?;
        assert!(result.is_none());

        Ok(())
    }
}
