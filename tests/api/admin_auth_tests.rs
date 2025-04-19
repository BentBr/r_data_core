// Basic test setup for Admin Auth routes
// More comprehensive tests to be implemented

#[cfg(test)]
mod tests {
    use r_data_core::{
        entity::admin_user::{AdminUserRepository, AdminUserRepositoryTrait},
        error::Result,
    };
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_admin_user_last_login_update() -> Result<()> {
        // Setup test database
        let pool = crate::common::setup_test_db().await;
        pool.begin().await?;

        // Create a repository to work with admin users directly
        let repo = AdminUserRepository::new(Arc::new(pool.clone()));

        // Generate a username and email that won't conflict
        let _unique_id = Uuid::now_v7().to_string()[0..8].to_string();
        // These variables are not used but kept as reference for more complete tests
        let _username = format!("test_user_{}", _unique_id);
        let _email = format!("test{}@example.com", _unique_id);
        
        // Password for reference in future tests
        let _test_password = "Test123!";
        
        // Create the admin user directly in the database
        let user_uuid = crate::common::create_test_admin_user(&pool).await?;

        // Verify the user has no last_login timestamp initially
        let initial_user = repo.find_by_uuid(&user_uuid).await?.unwrap();
        assert!(
            initial_user.last_login.is_none(),
            "New user should have no last_login timestamp"
        );

        // Simulate a login by updating the last_login timestamp
        repo.update_last_login(&user_uuid).await?;

        // Fetch the user again to check if last_login was updated
        let updated_user = repo.find_by_uuid(&user_uuid).await?.unwrap();
        assert!(
            updated_user.last_login.is_some(),
            "User's last_login should be updated after login"
        );

        // For a more comprehensive test, we would also verify JWT creation and validation
        // This would require calling the actual login endpoint, which would be part of an API test

        Ok(())
    }
} 