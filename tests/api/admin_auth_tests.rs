// Basic test setup for Admin Auth routes
// More comprehensive tests to be implemented

#[cfg(test)]
mod tests {
    use r_data_core::{
        entity::admin_user::{AdminUserRepository, AdminUserRepositoryTrait},
    };
    use r_data_core_core::error::Result;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_admin_user_last_login_update() -> r_data_core_core::error::Result<()> {
        // Setup test database
        let pool = crate::common::utils::setup_test_db().await;

        // Clear any existing data
        crate::common::utils::clear_test_db(&pool)
            .await
            .expect("Failed to clear test database");

        // Create a repository to work with admin users directly
        let repo = AdminUserRepository::new(Arc::new(pool.clone()));

        // Generate a username and email that won't conflict
        let unique_id = Uuid::now_v7().to_string()[0..8].to_string();
        let username = format!("test_user_{}", unique_id);
        let email = format!("test{}@example.com", unique_id);
        let test_password = "Test123!";

        // Create the admin user directly in the database
        let user_uuid = repo
            .create_admin_user(
                &username,
                &email,
                test_password,
                "Test",
                "User",
                None,
                true,
                Uuid::now_v7(),
            )
            .await?;

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
