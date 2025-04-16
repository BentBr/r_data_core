use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::entity::AdminUser;

#[async_trait]
pub trait AdminUserRepository {
    async fn find_by_username_or_email(
        &self,
        username_or_email: &str,
    ) -> sqlx::Result<Option<AdminUser>>;
    async fn update_last_login(&self, uuid: &Uuid) -> sqlx::Result<()>;
    async fn create_admin_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        first_name: &str,
        last_name: &str,
        role: Option<&str>,
        is_authenticated: bool,
        creator_uuid: Uuid,
    ) -> sqlx::Result<Uuid>;
}

pub struct PgAdminUserRepository {
    pool: PgPool,
}

impl PgAdminUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdminUserRepository for PgAdminUserRepository {
    async fn find_by_username_or_email(
        &self,
        username_or_email: &str,
    ) -> sqlx::Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_users WHERE username = $1 OR email = $1",
        )
        .bind(username_or_email)
        .fetch_optional(&self.pool)
        .await
    }

    async fn update_last_login(&self, uuid: &Uuid) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE admin_users SET last_login = NOW(), updated_at = NOW() WHERE uuid = $1",
        )
        .bind(uuid)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_admin_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        first_name: &str,
        last_name: &str,
        role: Option<&str>,
        is_authenticated: bool,
        creator_uuid: Uuid,
    ) -> sqlx::Result<Uuid> {
        // Create UUID for new user
        let user_uuid = Uuid::now_v7();

        // Set default values based on authentication status
        let is_active = is_authenticated; // Active if created by an authenticated user
        let published = is_authenticated; // Published if created by an authenticated user
        let path = "/users";

        // Hash the password using Argon2
        let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to hash password: {}", e)))?
            .to_string();

        // For authenticated requests, use the authenticated user's UUID as creator
        // For unauthenticated requests or if creator_uuid is nil, use the new user's UUID
        let created_by = if !creator_uuid.is_nil() {
            creator_uuid
        } else {
            user_uuid // Self-reference for unauthenticated registrations
        };

        // Insert the new admin user
        sqlx::query(
            "INSERT INTO admin_users (
                uuid, path, username, email, password_hash, first_name, last_name,
                is_active, created_at, updated_at, published, version, 
                created_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, 
                $8, NOW(), NOW(), $9, 1, 
                $10
            ) RETURNING uuid",
        )
        .bind(user_uuid)
        .bind(path)
        .bind(username)
        .bind(email)
        .bind(&password_hash)
        .bind(first_name)
        .bind(last_name)
        .bind(is_active)
        .bind(published)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        // In the future, we'd store the role in a separate table or add it to this table
        // For now, we'll ignore the role parameter

        Ok(user_uuid)
    }
}
