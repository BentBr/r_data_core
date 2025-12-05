#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

// Basic test setup for Admin Auth routes
// More comprehensive tests to be implemented

#[cfg(test)]
mod tests {
    use r_data_core_persistence::{
        AdminUserRepository, AdminUserRepositoryTrait, CreateAdminUserParams,
    };
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_admin_user_last_login_update() -> r_data_core_core::error::Result<()> {
        // Setup test database
        use r_data_core_test_support::{clear_test_db, setup_test_db};

        let pool = setup_test_db().await;

        // Clear any existing data
        clear_test_db(&pool)
            .await
            .expect("Failed to clear test database");

        // Create a repository to work with admin users directly
        let repo = AdminUserRepository::new(Arc::new(pool.clone()));

        // Generate a username and email that won't conflict
        let unique_id = Uuid::now_v7().to_string()[0..8].to_string();
        let username = format!("test_user_{unique_id}");
        let email = format!("test{unique_id}@example.com");
        let test_password = "Test123!";

        // Create the admin user directly in the database
        let params = CreateAdminUserParams {
            username: &username,
            email: &email,
            password: test_password,
            first_name: "Test",
            last_name: "User",
            role: None,
            is_active: true,
            creator_uuid: Uuid::now_v7(),
        };
        let user_uuid = repo.create_admin_user(&params).await?;

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
