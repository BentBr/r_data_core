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

        let now = OffsetDateTime::now_utc();

        // Set default values
        let published = is_active; // Published if active
        let path = "/admin/users";

        // Hash the password using Argon2
        let password_hash = hash_password(password)?;

        // Determine created_by value
        // If creator_uuid is nil, we'll use self-reference (update after insert)
        let is_self_creation = creator_uuid.is_nil();
        let initial_created_by = creator_uuid;

        // Insert the new admin user
        let user_uuid = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO admin_users (
                path, username, email, password_hash, first_name, last_name,
                is_active, created_at, updated_at, published, version, 
                created_by, super_admin
            ) VALUES (
                $1, $2, $3, $4, $5, $6, 
                $7, $8, $8, $9, 1, 
                $10, $11
            ) RETURNING uuid",
        )
        .bind(path)
        .bind(username)
        .bind(email)
        .bind(&password_hash)
        .bind(first_name)
        .bind(last_name)
        .bind(is_active)
        .bind(now)
        .bind(published)
        .bind(initial_created_by)
        .bind(false) // Default super_admin to false
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Error creating admin user: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })?;

        // If this is self-creation (no creator provided), update created_by to self-reference
        if is_self_creation {
            sqlx::query("UPDATE admin_users SET created_by = $1 WHERE uuid = $1")
                .bind(user_uuid)
                .execute(&*self.pool)
                .await
                .map_err(|e| {
                    error!("Error updating created_by for self-creation: {e:?}");
                    r_data_core_core::error::Error::Database(e)
                })?;
        }

        // In the future, we'd store the role in a separate table or add it to this table
        // For now, we'll ignore the role parameter

        Ok(user_uuid)
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
                password_hash = $7,
                updated_at = $8,
                version = version + 1
            WHERE uuid = $9",
        )
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.first_name)
        .bind(&user.last_name)
        .bind(user.is_active)
        .bind(user.super_admin)
        .bind(&user.password_hash)
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

    async fn list_admin_users(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<AdminUser>> {
        // Build ORDER BY clause - field is already validated and sanitized by route handler
        let order_by = match sort_by.as_deref() {
            // Virtual field: sort by number of roles assigned to the user
            Some("roles") => {
                let order = sort_order
                    .as_ref()
                    .map(|o| o.to_uppercase())
                    .filter(|o| o == "ASC" || o == "DESC")
                    .unwrap_or_else(|| "ASC".to_string());
                format!(
                    "(SELECT COUNT(*) FROM user_roles ur WHERE ur.user_uuid = admin_users.uuid) {order}"
                )
            }
            _ => sort_by.map_or_else(
                || "\"created_at\" DESC".to_string(),
                |field| {
                    // Field name is validated, but we still quote it for safety
                    let quoted_field = format!("\"{}\"", field.replace('"', "\"\""));
                    let order = sort_order
                        .as_ref()
                        .map(|o| o.to_uppercase())
                        .filter(|o| o == "ASC" || o == "DESC")
                        .unwrap_or_else(|| "ASC".to_string());
                    format!("{quoted_field} {order}")
                },
            ),
        };

        // Build query with or without LIMIT
        let query = if limit == i64::MAX {
            format!("SELECT * FROM admin_users ORDER BY {order_by} OFFSET $1")
        } else {
            format!("SELECT * FROM admin_users ORDER BY {order_by} LIMIT $1 OFFSET $2")
        };

        let mut query_builder = sqlx::query_as::<_, AdminUser>(&query);
        if limit == i64::MAX {
            query_builder = query_builder.bind(offset);
        } else {
            query_builder = query_builder.bind(limit).bind(offset);
        }

        query_builder.fetch_all(&*self.pool).await.map_err(|e| {
            error!("Error listing admin users: {e:?}");
            r_data_core_core::error::Error::Database(e)
        })
    }
}
