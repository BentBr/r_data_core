use crate::admin_user_repository_trait::AdminUserRepositoryTrait;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use async_trait::async_trait;
use log::error;
use r_data_core_core::admin_user::AdminUser;
use r_data_core_core::error::Result;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Repository for admin user operations
pub struct AdminUserRepository {
    pool: Arc<Pool<Postgres>>,
}

impl AdminUserRepository {
    /// Create a new repository instance
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const due to Arc parameter
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }

    /// Get all permission schemes assigned to a user
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_user_permission_schemes(&self, user_uuid: Uuid) -> Result<Vec<Uuid>> {
        let schemes = sqlx::query_scalar::<_, Uuid>(
            "SELECT scheme_uuid FROM admin_users_permission_schemes WHERE user_uuid = $1",
        )
        .bind(user_uuid)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error getting user permission schemes: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(schemes)
    }

    /// Assign a permission scheme to a user
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn assign_permission_scheme(&self, user_uuid: Uuid, scheme_uuid: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT INTO admin_users_permission_schemes (user_uuid, scheme_uuid) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(user_uuid)
        .bind(scheme_uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error assigning permission scheme to user: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    /// Unassign a permission scheme from a user
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn unassign_permission_scheme(
        &self,
        user_uuid: Uuid,
        scheme_uuid: Uuid,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM admin_users_permission_schemes WHERE user_uuid = $1 AND scheme_uuid = $2",
        )
        .bind(user_uuid)
        .bind(scheme_uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error unassigning permission scheme from user: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    /// Update all permission schemes for a user (replace existing assignments)
    ///
    /// # Errors
    /// Returns an error if the database transaction fails
    pub async fn update_user_schemes(&self, user_uuid: Uuid, scheme_uuids: &[Uuid]) -> Result<()> {
        // Start a transaction
        let mut tx = self.pool.begin().await.map_err(|e| {
            error!("Error starting transaction: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        // Delete all existing assignments
        sqlx::query("DELETE FROM admin_users_permission_schemes WHERE user_uuid = $1")
            .bind(user_uuid)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Error deleting existing permission schemes: {e:?}");
                r_data_core_core::error::Error::Database(e)
            })?;

        // Insert new assignments
        for scheme_uuid in scheme_uuids {
            sqlx::query(
                "INSERT INTO admin_users_permission_schemes (user_uuid, scheme_uuid) VALUES ($1, $2)",
            )
            .bind(user_uuid)
            .bind(scheme_uuid)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Error assigning permission scheme: {e:?}");
                r_data_core_core::error::Error::Database(e)
            })?;
        }

        // Commit transaction
        tx.commit().await.map_err(|e| {
            error!("Error committing transaction: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }
}

#[async_trait]
impl AdminUserRepositoryTrait for AdminUserRepository {
    async fn find_by_username_or_email(
        &self,
        username_or_email: &str,
    ) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_users WHERE username = $1 OR email = $1",
        )
        .bind(username_or_email)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error finding user by username or email: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })
    }

    async fn find_by_uuid(&self, uuid: &Uuid) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>("SELECT * FROM admin_users WHERE uuid = $1")
            .bind(uuid)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| {
                error!("Error finding user by UUID: {e:?}");
                r_data_core_core::error::Error::Database(e)
            })
    }

    async fn update_last_login(&self, uuid: &Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE admin_users SET last_login = NOW(), updated_at = NOW() WHERE uuid = $1",
        )
        .bind(uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error updating last login: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    async fn create_admin_user<'a>(
        &self,
        username: &str,
        email: &str,
        password: &str,
        first_name: &str,
        last_name: &str,
        _role: Option<&'a str>,
        is_active: bool,
        creator_uuid: Uuid,
    ) -> Result<Uuid> {
        // Create UUID for the new user
        let user_uuid = Uuid::now_v7();
        let now = OffsetDateTime::now_utc();

        // Set default values
        let published = is_active; // Published if active
        let path = "/admin/users";

        // Hash the password using Argon2
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| {
                r_data_core_core::error::Error::PasswordHash(format!(
                    "Failed to hash password: {e}"
                ))
            })?
            .to_string();

        // For authenticated requests, use the authenticated user's UUID as creator
        // For unauthenticated requests or if creator_uuid is nil, use the new user's UUID
        let created_by = if creator_uuid != Uuid::nil() {
            creator_uuid
        } else {
            user_uuid // Self-reference for unauthenticated registrations
        };

        // Insert the new admin user
        let result = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO admin_users (
                uuid, path, username, email, password_hash, first_name, last_name,
                is_active, created_at, updated_at, published, version, 
                created_by, super_admin
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, 
                $8, $9, $9, $10, 1, 
                $11, $12
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
        .bind(now)
        .bind(published)
        .bind(created_by)
        .bind(false) // Default super_admin to false
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error creating admin user: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        // In the future, we'd store the role in a separate table or add it to this table
        // For now, we'll ignore the role parameter

        Ok(result)
    }

    async fn update_admin_user(&self, user: &AdminUser) -> Result<()> {
        sqlx::query(
            "UPDATE admin_users SET 
                username = $1, 
                email = $2, 
                first_name = $3, 
                last_name = $4, 
                is_active = $5, 
                super_admin = $6,
                updated_at = $7,
                version = version + 1
            WHERE uuid = $8",
        )
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.first_name)
        .bind(&user.last_name)
        .bind(user.is_active)
        .bind(user.super_admin)
        .bind(OffsetDateTime::now_utc())
        .bind(user.uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error updating admin user: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    async fn delete_admin_user(&self, uuid: &Uuid) -> Result<()> {
        // Soft delete by setting is_active to false
        sqlx::query!(
            "UPDATE admin_users SET is_active = false, updated_at = $1 WHERE uuid = $2",
            OffsetDateTime::now_utc(),
            uuid
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error deleting admin user: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    async fn list_admin_users(&self, limit: i64, offset: i64) -> Result<Vec<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error listing admin users: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })
    }
}
