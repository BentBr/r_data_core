#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use base64::Engine as _;
use r_data_core_core::error::{Error, Result};
use r_data_core_persistence::{EmailTemplateRepositoryTrait, PasswordResetRepositoryTrait};
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::SendEmailJob;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

/// Service for handling the password reset flow.
pub struct PasswordResetService {
    repo: Arc<dyn PasswordResetRepositoryTrait>,
    user_repo: Arc<dyn r_data_core_persistence::AdminUserRepositoryTrait>,
    template_repo: Arc<dyn EmailTemplateRepositoryTrait>,
    queue: Arc<dyn JobQueue>,
    mail_service: Arc<crate::mail::MailService>,
    throttle_seconds: u64,
    frontend_base_url: String,
}

impl PasswordResetService {
    /// Create a new [`PasswordResetService`].
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        repo: Arc<dyn PasswordResetRepositoryTrait>,
        user_repo: Arc<dyn r_data_core_persistence::AdminUserRepositoryTrait>,
        template_repo: Arc<dyn EmailTemplateRepositoryTrait>,
        queue: Arc<dyn JobQueue>,
        mail_service: Arc<crate::mail::MailService>,
        throttle_seconds: u64,
        frontend_base_url: String,
    ) -> Self {
        Self {
            repo,
            user_repo,
            template_repo,
            queue,
            mail_service,
            throttle_seconds,
            frontend_base_url,
        }
    }

    /// Request a password reset for the given email address.
    ///
    /// Always returns `Ok` so that callers cannot determine whether a given
    /// email exists in the system.
    ///
    /// # Errors
    ///
    /// Returns an error only on unexpected infrastructure failures (database,
    /// queue, etc.).  A missing user or a throttled request are silently
    /// ignored.
    pub async fn request_reset(&self, email: &str) -> Result<()> {
        // Look up user — silently return if not found to avoid leaking existence.
        let Some(user) = self.user_repo.find_by_username_or_email(email).await? else {
            return Ok(());
        };

        // Throttle: if the most recent token was created within `throttle_seconds`, bail out.
        if let Some(latest) = self.repo.find_latest_for_user(user.uuid).await? {
            let age = OffsetDateTime::now_utc() - latest.created_at;
            #[allow(clippy::cast_possible_wrap)]
            let throttle_secs = self.throttle_seconds as i64;
            if age.whole_seconds() < throttle_secs {
                return Ok(());
            }
        }

        // Generate a secure random token (32 bytes, base64url-encoded).
        let token = {
            use rand::Rng as _;
            let mut rng = rand::rng();
            let bytes: [u8; 32] = rng.random();
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
        };

        // Hash the token for storage.
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

        // Delete old tokens for this user and insert the new one.
        self.repo.delete_for_user(user.uuid).await?;

        let expires_at = OffsetDateTime::now_utc() + time::Duration::hours(1);
        self.repo
            .insert_token(user.uuid, &token_hash, expires_at)
            .await?;

        // Build the reset URL.
        let reset_url = format!("{}/reset-password?token={token}", self.frontend_base_url);

        // Load the email template.
        let Some(template) = self.template_repo.get_by_slug("password_reset").await? else {
            return Err(Error::Config(
                "Email template 'password_reset' not found".to_string(),
            ));
        };

        // Build the template context.
        let context = serde_json::json!({
            "reset_url": reset_url,
            "user_name": user.full_name,
            "user_email": user.email,
        });

        // Render subject and bodies.
        let subject = self
            .mail_service
            .render_template(&template.subject_template, &context)?;
        let body_text = self
            .mail_service
            .render_template(&template.body_text_template, &context)?;
        let body_html = self
            .mail_service
            .render_template(&template.body_html_template, &context)?;

        // Enqueue the email job.
        let job = SendEmailJob {
            run_uuid: None,
            to: vec![user.email.clone()],
            cc: vec![],
            subject,
            body_text,
            body_html: Some(body_html),
            from_name_override: None,
            source: "system".to_string(),
            template_uuid: Some(template.uuid),
            template_context: None,
        };

        self.queue.enqueue_email(job).await?;

        Ok(())
    }

    /// Validate a password-reset token and update the user's password.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if the token is invalid, expired, or
    /// already used.  Returns other errors on infrastructure failures.
    pub async fn reset_password(&self, token: &str, new_password: &str) -> Result<()> {
        // Hash the provided token to look it up.
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

        // Fetch the stored token record.
        let record = self
            .repo
            .find_by_token_hash(&token_hash)
            .await?
            .ok_or_else(|| {
                Error::Validation("Invalid or expired password reset token".to_string())
            })?;

        // Check expiry.
        if record.expires_at < OffsetDateTime::now_utc() {
            return Err(Error::Validation(
                "Password reset token has expired".to_string(),
            ));
        }

        // Check not already used.
        if record.used_at.is_some() {
            return Err(Error::Validation(
                "Password reset token has already been used".to_string(),
            ));
        }

        // Hash the new password.
        let new_hash = r_data_core_core::crypto::hash_password_argon2(new_password)?;

        // Load the user and update the password hash in-place.
        let mut user = self
            .user_repo
            .find_by_uuid(&record.user_id)
            .await?
            .ok_or_else(|| Error::Validation("User not found".to_string()))?;

        user.password_hash = new_hash;

        self.user_repo.update_admin_user(&user).await?;

        // Mark token as used.
        self.repo.mark_used(record.id).await?;

        Ok(())
    }
}
