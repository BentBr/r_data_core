#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use r_data_core_core::error::Result;
use r_data_core_core::password_reset_token::PasswordResetToken;
use time::OffsetDateTime;
use uuid::Uuid;

/// Trait for password reset token repository operations
#[async_trait]
pub trait PasswordResetRepositoryTrait: Send + Sync {
    /// Insert a new password reset token
    ///
    /// # Errors
    /// Returns an error if the database insert fails
    async fn insert_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: OffsetDateTime,
    ) -> Result<Uuid>;

    /// Find a password reset token by its hash
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn find_by_token_hash(&self, hash: &str) -> Result<Option<PasswordResetToken>>;

    /// Find the latest password reset token for a user
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn find_latest_for_user(&self, user_id: Uuid) -> Result<Option<PasswordResetToken>>;

    /// Mark a password reset token as used
    ///
    /// # Errors
    /// Returns an error if the database update fails
    async fn mark_used(&self, id: Uuid) -> Result<()>;

    /// Delete all expired password reset tokens
    ///
    /// # Errors
    /// Returns an error if the database delete fails
    async fn delete_expired(&self) -> Result<u64>;

    /// Delete all password reset tokens for a user
    ///
    /// # Errors
    /// Returns an error if the database delete fails
    async fn delete_for_user(&self, user_id: Uuid) -> Result<()>;
}
