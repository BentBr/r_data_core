use crate::common::utils;
use r_data_core::{
    entity::admin_user::{AdminUserRepository, AdminUserRepositoryTrait},
    error::Result,
};
use serial_test::serial;
use std::sync::Arc;

#[tokio::test]
#[serial]
async fn test_create_and_find_admin_user() -> Result<()> {
    // Setup
    let pool = utils::setup_test_db().await;
    utils::clear_test_db(&pool).await?;

    let repo = AdminUserRepository::new(Arc::new(pool.clone()));

    let username = utils::random_string("test_user");
    let email = format!("{}@example.com", username);
    let first_name = "Test";
    let last_name = "User";
    let password = "password123";

    // Create a user to serve as the creator
    let creator_uuid = utils::create_test_admin_user(&pool).await?;

    // Create the test user
    let user_uuid = repo
        .create_admin_user(
            &username,
            &email,
            password,
            first_name,
            last_name,
            None,
            true, // is_active
            creator_uuid,
        )
        .await?;

    // Find the user by username
    let found_user = repo.find_by_username_or_email(&username).await?;

    // Verify
    assert!(found_user.is_some(), "User should be findable by username");
    let user = found_user.unwrap();
    assert_eq!(user.uuid, user_uuid);
    assert_eq!(user.username, username);
    assert_eq!(user.email, email);
    assert_eq!(user.first_name.unwrap(), first_name);
    assert_eq!(user.last_name.unwrap(), last_name);
    assert!(user.is_active);

    // Also find by UUID
    let found_by_uuid = repo.find_by_uuid(&user_uuid).await?;
    assert!(
        found_by_uuid.is_some(),
        "User with UUID {} should exist",
        user_uuid
    );

    // Update last login
    repo.update_last_login(&user_uuid).await?;

    // Verify the last login was updated
    let updated_user = repo.find_by_uuid(&user_uuid).await?;
    assert!(
        updated_user.is_some(),
        "User should still exist after login update"
    );
    let updated_user = updated_user.unwrap();
    assert!(updated_user.last_login.is_some());

    // Delete (soft delete)
    repo.delete_admin_user(&user_uuid).await?;

    // Check that user is now inactive
    let deleted_user = repo.find_by_uuid(&user_uuid).await?;
    assert!(
        deleted_user.is_some(),
        "User should still exist after soft delete"
    );
    let deleted_user = deleted_user.unwrap();
    assert!(!deleted_user.is_active);

    Ok(())
}
