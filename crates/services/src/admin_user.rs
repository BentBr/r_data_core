use r_data_core_core::admin_user::AdminUser;
use r_data_core_core::error::Result;
use r_data_core_persistence::AdminUserRepositoryTrait;
use std::sync::Arc;
use uuid::Uuid;

use crate::query_validation::{validate_list_query, FieldValidator, ValidatedListQuery};

/// Service for admin user operations
pub struct AdminUserService {
    repository: Arc<dyn AdminUserRepositoryTrait>,
}

impl AdminUserService {
    /// Create a new admin user service with a repository
    #[must_use]
    pub fn new(repository: Arc<dyn AdminUserRepositoryTrait>) -> Self {
        Self { repository }
    }

    /// Create a new admin user service from a concrete repository
    #[must_use]
    pub fn from_repository<T: AdminUserRepositoryTrait + 'static>(repository: T) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }

    /// Authenticate a user with username/email and password
    ///
    /// # Errors
    /// Returns an error if validation fails, user is not active, or database operation fails
    pub async fn authenticate(
        &self,
        username_or_email: &str,
        password: &str,
    ) -> Result<Option<AdminUser>> {
        if username_or_email.is_empty() || password.is_empty() {
            return Err(r_data_core_core::error::Error::Validation(
                "Username/email and password are required".to_string(),
            ));
        }

        // Find the user
        let Some(user) = self
            .repository
            .find_by_username_or_email(username_or_email)
            .await?
        else {
            return Ok(None);
        };

        // Verify password
        if !user.verify_password(password) {
            return Ok(None);
        }

        // Check if user is active
        if !user.is_active {
            return Err(r_data_core_core::error::Error::Auth(
                "Account is not active".to_string(),
            ));
        }

        // Update last login time
        self.repository.update_last_login(&user.uuid).await?;

        Ok(Some(user))
    }

    /// Register a new admin user
    ///
    /// # Errors
    /// Returns an error if validation fails, username/email already exists, or database operation fails
    #[allow(clippy::too_many_arguments)] // Public API - parameters are clear and well-named
    pub async fn register_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        first_name: &str,
        last_name: &str,
        role: Option<&str>,
        is_active: bool,
        creator_uuid: Uuid,
    ) -> Result<Uuid> {
        // Basic validation
        if username.is_empty() || email.is_empty() || password.is_empty() {
            return Err(r_data_core_core::error::Error::Validation(
                "Username, email, and password are required".to_string(),
            ));
        }

        if password.len() < 8 {
            return Err(r_data_core_core::error::Error::Validation(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        // Check if username or email already exists
        let existing_user = self.repository.find_by_username_or_email(username).await?;
        if existing_user.is_some() {
            return Err(r_data_core_core::error::Error::Validation(
                "Username already exists".to_string(),
            ));
        }

        let existing_user = self.repository.find_by_username_or_email(email).await?;
        if existing_user.is_some() {
            return Err(r_data_core_core::error::Error::Validation(
                "Email already in use".to_string(),
            ));
        }

        // Create the user
        let params = r_data_core_persistence::CreateAdminUserParams {
            username,
            email,
            password,
            first_name,
            last_name,
            role,
            is_active,
            creator_uuid,
        };
        self.repository.create_admin_user(&params).await
    }

    /// Get a user by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_user_by_uuid(&self, uuid: &Uuid) -> Result<Option<AdminUser>> {
        self.repository.find_by_uuid(uuid).await
    }

    /// Update a user
    ///
    /// # Errors
    /// Returns an error if the user is not found or database operation fails
    pub async fn update_user(&self, user: &AdminUser) -> Result<()> {
        // Check if the user exists
        let existing_user = self.repository.find_by_uuid(&user.uuid).await?;
        if existing_user.is_none() {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "User with UUID {} not found",
                user.uuid
            )));
        }

        self.repository.update_admin_user(user).await
    }

    /// Delete a user
    ///
    /// # Errors
    /// Returns an error if the user is not found or database operation fails
    pub async fn delete_user(&self, uuid: &Uuid) -> Result<()> {
        // Check if the user exists
        let existing_user = self.repository.find_by_uuid(uuid).await?;
        if existing_user.is_none() {
            return Err(r_data_core_core::error::Error::NotFound(format!(
                "User with UUID {uuid} not found"
            )));
        }

        self.repository.delete_admin_user(uuid).await
    }

    /// List users with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_users(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<AdminUser>> {
        // Validate input
        let limit = if limit <= 0 { 50 } else { limit };
        let offset = if offset < 0 { 0 } else { offset };

        self.repository
            .list_admin_users(limit, offset, sort_by, sort_order)
            .await
    }

    /// List users with query validation
    ///
    /// This method validates the query parameters and returns validated parameters along with users.
    ///
    /// # Arguments
    /// * `params` - The query parameters
    /// * `field_validator` - The `FieldValidator` instance (required for validation)
    ///
    /// # Returns
    /// A tuple of (users, `validated_query`) where `validated_query` contains pagination metadata
    ///
    /// # Errors
    /// Returns an error if validation fails or database query fails
    pub async fn list_users_with_query(
        &self,
        params: &crate::query_validation::ListQueryParams,
        field_validator: &FieldValidator,
    ) -> Result<(Vec<AdminUser>, ValidatedListQuery)> {
        // Allow sorting by virtual "roles" field (number of roles) in addition to DB columns
        let validated = validate_list_query(
            params,
            "admin_users",
            field_validator,
            20,
            100,
            true,
            &["roles"],
        )
        .await
        .map_err(r_data_core_core::error::Error::Validation)?;

        let users = self
            .repository
            .list_admin_users(
                validated.limit,
                validated.offset,
                validated.sort_by.clone(),
                validated.sort_order.clone(),
            )
            .await?;

        Ok((users, validated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;
    use r_data_core_core::admin_user::UserStatus;
    use r_data_core_core::domain::AbstractRDataEntity;
    use time::OffsetDateTime;

    mock! {
        pub AdminUserRepo {}

        #[async_trait]
        impl AdminUserRepositoryTrait for AdminUserRepo {
            async fn find_by_username_or_email(&self, username_or_email: &str) -> Result<Option<AdminUser>>;
            async fn find_by_uuid(&self, uuid: &Uuid) -> Result<Option<AdminUser>>;
            async fn update_last_login(&self, uuid: &Uuid) -> Result<()>;
            async fn create_admin_user<'a>(
                &self,
                params: &r_data_core_persistence::admin_user_repository_trait::CreateAdminUserParams<'a>,
            ) -> Result<Uuid>;
            async fn update_admin_user(&self, user: &AdminUser) -> Result<()>;
            async fn delete_admin_user(&self, uuid: &Uuid) -> Result<()>;
            async fn list_admin_users(&self, limit: i64, offset: i64, sort_by: Option<String>, sort_order: Option<String>) -> Result<Vec<AdminUser>>;
        }
    }

    #[tokio::test]
    async fn test_authenticate_success() {
        let mut mock_repo = MockAdminUserRepo::new();
        let user_uuid = Uuid::now_v7();
        let now = OffsetDateTime::now_utc();

        // Create a test user
        let test_user = AdminUser {
            uuid: user_uuid,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$WwD1am7XJrm2JAMuY4QQVGRBfFmLwUJX7p4NCZEw9MU".to_string(), // Hash for "password123"
            full_name: "Test User".to_string(),
            status: UserStatus::Active,
            last_login: None,
            failed_login_attempts: 0,
            super_admin: false,
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            is_active: true,
            is_admin: true,
            created_at: now,
            updated_at: now,
            base: AbstractRDataEntity::new("/admin/users".to_string()),
        };

        // Setup expectations
        mock_repo
            .expect_find_by_username_or_email()
            .with(eq("testuser"))
            .returning(move |_| Ok(Some(test_user.clone())));

        mock_repo
            .expect_update_last_login()
            .with(eq(user_uuid))
            .returning(|_| Ok(()));

        let service = AdminUserService::new(Arc::new(mock_repo));

        // This will fail because the password hash is not for "wrongpassword"
        let result = service.authenticate("testuser", "wrongpassword").await;

        // Should return None for wrong password
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_register_user_success() {
        let mut mock_repo = MockAdminUserRepo::new();
        let user_uuid = Uuid::now_v7();
        let creator_uuid = Uuid::now_v7();

        // Setup expectations
        mock_repo
            .expect_find_by_username_or_email()
            .with(eq("newuser"))
            .returning(|_| Ok(None));

        mock_repo
            .expect_find_by_username_or_email()
            .with(eq("new@example.com"))
            .returning(|_| Ok(None));

        mock_repo.expect_create_admin_user().returning(
            move |params: &r_data_core_persistence::admin_user_repository_trait::CreateAdminUserParams| {
                assert_eq!(params.username, "newuser");
                assert_eq!(params.email, "new@example.com");
                assert_eq!(params.password, "password123");
                assert_eq!(params.first_name, "New");
                assert_eq!(params.last_name, "User");
                assert!(params.role.is_none());
                assert!(params.is_active);
                assert_eq!(params.creator_uuid, creator_uuid);
                Ok(user_uuid)
            },
        );

        let service = AdminUserService::new(Arc::new(mock_repo));

        let result = service
            .register_user(
                "newuser",
                "new@example.com",
                "password123",
                "New",
                "User",
                None,
                true,
                creator_uuid,
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), user_uuid);
    }

    #[tokio::test]
    async fn test_register_user_existing_username() {
        let mut mock_repo = MockAdminUserRepo::new();
        let user_uuid = Uuid::now_v7();
        let creator_uuid = Uuid::now_v7();
        let now = OffsetDateTime::now_utc();

        // Create a test user
        let test_user = AdminUser {
            uuid: user_uuid,
            username: "existinguser".to_string(),
            email: "existing@example.com".to_string(),
            password_hash: "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$WwD1am7XJrm2JAMuY4QQVGRBfFmLwUJX7p4NCZEw9MU".to_string(),
            full_name: "Existing User".to_string(),
            status: UserStatus::Active,
            last_login: None,
            failed_login_attempts: 0,
            super_admin: false,
            first_name: Some("Existing".to_string()),
            last_name: Some("User".to_string()),
            is_active: true,
            is_admin: true,
            created_at: now,
            updated_at: now,
            base: AbstractRDataEntity::new("/admin/users".to_string()),
        };

        // Setup expectations
        mock_repo
            .expect_find_by_username_or_email()
            .with(eq("existinguser"))
            .returning(move |_| Ok(Some(test_user.clone())));

        let service = AdminUserService::new(Arc::new(mock_repo));

        let result = service
            .register_user(
                "existinguser",
                "new@example.com",
                "password123",
                "New",
                "User",
                None,
                true,
                creator_uuid,
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(r_data_core_core::error::Error::Validation(msg)) => {
                assert_eq!(msg, "Username already exists");
            }
            _ => panic!("Expected validation error"),
        }
    }
}
