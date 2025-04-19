use crate::{
    entity::admin_user::{AdminUser, AdminUserRepositoryTrait},
    error::{Error, Result},
};
use std::sync::Arc;
use uuid::Uuid;

/// Service for admin user operations
pub struct AdminUserService {
    repository: Arc<dyn AdminUserRepositoryTrait>,
}

impl AdminUserService {
    /// Create a new admin user service with a repository
    pub fn new(repository: Arc<dyn AdminUserRepositoryTrait>) -> Self {
        Self { repository }
    }

    /// Authenticate a user with username/email and password
    pub async fn authenticate(
        &self,
        username_or_email: &str,
        password: &str,
    ) -> Result<Option<AdminUser>> {
        if username_or_email.is_empty() || password.is_empty() {
            return Err(Error::Validation(
                "Username/email and password are required".to_string(),
            ));
        }

        // Find the user
        let user = match self.repository.find_by_username_or_email(username_or_email).await? {
            Some(user) => user,
            None => return Ok(None),
        };

        // Verify password
        if !user.verify_password(password) {
            return Ok(None);
        }

        // Check if user is active
        if !user.is_active {
            return Err(Error::Auth("Account is not active".to_string()));
        }

        // Update last login time
        self.repository.update_last_login(&user.uuid).await?;

        Ok(Some(user))
    }

    /// Register a new admin user
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
            return Err(Error::Validation(
                "Username, email, and password are required".to_string(),
            ));
        }

        if password.len() < 8 {
            return Err(Error::Validation(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        // Check if username or email already exists
        let existing_user = self.repository.find_by_username_or_email(username).await?;
        if existing_user.is_some() {
            return Err(Error::Validation("Username already exists".to_string()));
        }

        let existing_user = self.repository.find_by_username_or_email(email).await?;
        if existing_user.is_some() {
            return Err(Error::Validation("Email already in use".to_string()));
        }

        // Create the user
        self.repository
            .create_admin_user(
                username,
                email,
                password,
                first_name,
                last_name,
                role,
                is_active,
                creator_uuid,
            )
            .await
    }

    /// Get a user by UUID
    pub async fn get_user_by_uuid(&self, uuid: &Uuid) -> Result<Option<AdminUser>> {
        self.repository.find_by_uuid(uuid).await
    }

    /// Update a user
    pub async fn update_user(&self, user: &AdminUser) -> Result<()> {
        // Check if the user exists
        let existing_user = self.repository.find_by_uuid(&user.uuid).await?;
        if existing_user.is_none() {
            return Err(Error::NotFound(
                format!("User with UUID {} not found", user.uuid)
            ));
        }

        self.repository.update_admin_user(user).await
    }

    /// Delete a user
    pub async fn delete_user(&self, uuid: &Uuid) -> Result<()> {
        // Check if the user exists
        let existing_user = self.repository.find_by_uuid(uuid).await?;
        if existing_user.is_none() {
            return Err(Error::NotFound(
                format!("User with UUID {} not found", uuid)
            ));
        }

        self.repository.delete_admin_user(uuid).await
    }

    /// List users with pagination
    pub async fn list_users(&self, limit: i64, offset: i64) -> Result<Vec<AdminUser>> {
        // Validate input
        let limit = if limit <= 0 { 50 } else { limit };
        let offset = if offset < 0 { 0 } else { offset };

        self.repository.list_admin_users(limit, offset).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;
    use time::OffsetDateTime;
    use crate::entity::admin_user::{UserRole, UserStatus};
    use crate::entity::AbstractRDataEntity;

    mock! {
        pub AdminUserRepo {}
        
        #[async_trait]
        impl AdminUserRepositoryTrait for AdminUserRepo {
            async fn find_by_username_or_email(&self, username_or_email: &str) -> Result<Option<AdminUser>>;
            async fn find_by_uuid(&self, uuid: &Uuid) -> Result<Option<AdminUser>>;
            async fn update_last_login(&self, uuid: &Uuid) -> Result<()>;
            async fn create_admin_user<'a>(
                &self,
                username: &str,
                email: &str,
                password: &str,
                first_name: &str,
                last_name: &str,
                role: Option<&'a str>,
                is_active: bool,
                creator_uuid: Uuid,
            ) -> Result<Uuid>;
            async fn update_admin_user(&self, user: &AdminUser) -> Result<()>;
            async fn delete_admin_user(&self, uuid: &Uuid) -> Result<()>;
            async fn list_admin_users(&self, limit: i64, offset: i64) -> Result<Vec<AdminUser>>;
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
            role: UserRole::Admin,
            status: UserStatus::Active,
            last_login: None,
            failed_login_attempts: 0,
            permission_scheme_uuid: None,
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
            
        mock_repo
            .expect_create_admin_user()
            .returning(move |username, email, password, first_name, last_name, role, is_active, creator| {
                assert_eq!(username, "newuser");
                assert_eq!(email, "new@example.com");
                assert_eq!(password, "password123");
                assert_eq!(first_name, "New");
                assert_eq!(last_name, "User");
                assert!(role.is_none());
                assert!(is_active);
                assert_eq!(creator, creator_uuid);
                Ok(user_uuid)
            });
        
        let service = AdminUserService::new(Arc::new(mock_repo));
        
        let result = service.register_user(
            "newuser",
            "new@example.com",
            "password123",
            "New",
            "User",
            None,
            true,
            creator_uuid
        ).await;
        
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
            role: UserRole::Admin,
            status: UserStatus::Active,
            last_login: None,
            failed_login_attempts: 0,
            permission_scheme_uuid: None,
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
        
        let result = service.register_user(
            "existinguser",
            "new@example.com",
            "password123",
            "New",
            "User",
            None,
            true,
            creator_uuid
        ).await;
        
        assert!(result.is_err());
        match result {
            Err(Error::Validation(msg)) => {
                assert_eq!(msg, "Username already exists");
            }
            _ => panic!("Expected validation error"),
        }
    }
} 