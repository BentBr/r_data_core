#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use r_data_core_core::email_template::{EmailTemplate, EmailTemplateType};
use r_data_core_core::error::Result;
use uuid::Uuid;

/// Trait for email template repository operations
#[async_trait]
pub trait EmailTemplateRepositoryTrait: Send + Sync {
    /// List all email templates
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn list_all(&self) -> Result<Vec<EmailTemplate>>;

    /// List email templates filtered by type
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn list_by_type(&self, template_type: EmailTemplateType) -> Result<Vec<EmailTemplate>>;

    /// Get an email template by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<EmailTemplate>>;

    /// Get an email template by slug
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn get_by_slug(&self, slug: &str) -> Result<Option<EmailTemplate>>;

    /// Create a new email template
    ///
    /// # Errors
    /// Returns an error if the database insert fails
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        name: &str,
        slug: &str,
        template_type: EmailTemplateType,
        subject: &str,
        body_html: &str,
        body_text: &str,
        variables: serde_json::Value,
        created_by: Uuid,
    ) -> Result<Uuid>;

    /// Update an existing email template
    ///
    /// # Errors
    /// Returns an error if the database update fails
    #[allow(clippy::too_many_arguments)]
    async fn update(
        &self,
        uuid: Uuid,
        name: Option<&str>,
        subject: &str,
        body_html: &str,
        body_text: &str,
        variables: serde_json::Value,
        updated_by: Uuid,
    ) -> Result<()>;

    /// Delete an email template by UUID
    ///
    /// # Errors
    /// Returns an error if the database delete fails
    async fn delete(&self, uuid: Uuid) -> Result<()>;
}
