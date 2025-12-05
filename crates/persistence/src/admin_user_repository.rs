use crate::admin_user_repository_trait::AdminUserRepositoryTrait;
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

/// Hash a password using Argon2
fn hash_password(password: &str) -> Result<String> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| {
            r_data_core_core::error::Error::PasswordHash(format!("Failed to hash password: {e}"))
        })
        .map(|hash| hash.to_string())
}

/// Determine the `created_by` UUID value
/// For authenticated requests, use the authenticated user's UUID as creator
/// For unauthenticated requests or if `creator_uuid` is nil, use the new user's UUID
const fn determine_created_by(creator_uuid: Uuid, user_uuid: Uuid) -> Uuid {
    if creator_uuid.is_nil() {
        user_uuid // Self-reference for unauthenticated registrations
    } else {
        creator_uuid
    }
}

impl AdminUserRepository {
    /// Create a new repository instance
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const due to Arc parameter
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }

    /// Get all roles assigned to a user
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_user_roles(&self, user_uuid: Uuid) -> Result<Vec<Uuid>> {
        let roles =
            sqlx::query_scalar::<_, Uuid>("SELECT role_uuid FROM user_roles WHERE user_uuid = $1")
                .bind(user_uuid)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| {
                    error!("Error getting user roles: {e:?}");
                    r_data_core_core::error::Error::Database(e)
                })?;

        Ok(roles)
    }

    /// Get all users that have a specific role assigned
    ///
    /// # Arguments
    /// * `role_uuid` - Role UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get_users_by_role(&self, role_uuid: Uuid) -> Result<Vec<Uuid>> {
        let users =
            sqlx::query_scalar::<_, Uuid>("SELECT user_uuid FROM user_roles WHERE role_uuid = $1")
                .bind(role_uuid)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| {
                    error!("Error getting users by role: {e:?}");
                    r_data_core_core::error::Error::Database(e)
                })?;

        Ok(users)
    }

    /// Assign a role to a user
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn assign_role(&self, user_uuid: Uuid, role_uuid: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_roles (user_uuid, role_uuid) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(user_uuid)
        .bind(role_uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error assigning role to user: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        Ok(())
    }

    /// Unassign a role from a user
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn unassign_role(&self, user_uuid: Uuid, role_uuid: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM user_roles WHERE user_uuid = $1 AND role_uuid = $2")
            .bind(user_uuid)
            .bind(role_uuid)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                error!("Error unassigning role from user: {e:?}");
                r_data_core_core::error::Error::Database(e)
            })?;

        Ok(())
    }

    /// Update all roles for a user (replace existing assignments)
    ///
    /// # Errors
    /// Returns an error if the database transaction fails
    pub async fn update_user_roles(&self, user_uuid: Uuid, role_uuids: &[Uuid]) -> Result<()> {
        // Start a transaction
        let mut tx = self.pool.begin().await.map_err(|e| {
            error!("Error starting transaction: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        // Delete all existing assignments
        sqlx::query("DELETE FROM user_roles WHERE user_uuid = $1")
            .bind(user_uuid)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Error deleting existing roles: {e:?}");
                r_data_core_core::error::Error::Database(e)
            })?;

        // Insert new assignments
        for role_uuid in role_uuids {
            sqlx::query("INSERT INTO user_roles (user_uuid, role_uuid) VALUES ($1, $2)")
                .bind(user_uuid)
                .bind(role_uuid)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error!("Error assigning role: {e:?}");
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
        params: &crate::admin_user_repository_trait::CreateAdminUserParams<'a>,
    ) -> Result<Uuid> {
        let username = params.username;
        let email = params.email;
        let password = params.password;
        let first_name = params.first_name;
        let last_name = params.last_name;
        let _role = params.role;
        let is_active = params.is_active;
        let creator_uuid = params.creator_uuid;

        // Create UUID for the new user
        let user_uuid = Uuid::now_v7();
        let now = OffsetDateTime::now_utc();

        // Set default values
        let published = is_active; // Published if active
        let path = "/admin/users";

        // Hash the password using Argon2
        let password_hash = hash_password(password)?;

        // Determine created_by value
        let created_by = determine_created_by(creator_uuid, user_uuid);

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
