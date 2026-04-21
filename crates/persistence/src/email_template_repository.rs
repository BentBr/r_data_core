#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::email_template_repository_trait::EmailTemplateRepositoryTrait;
use r_data_core_core::email_template::{EmailTemplate, EmailTemplateType};
use r_data_core_core::error::Result;

/// Repository for email template operations
pub struct EmailTemplateRepository {
    pool: PgPool,
}

impl EmailTemplateRepository {
    /// Create a new email template repository
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmailTemplateRepositoryTrait for EmailTemplateRepository {
    async fn list_all(&self) -> Result<Vec<EmailTemplate>> {
        let rows = sqlx::query_as!(
            EmailTemplate,
            r#"
            SELECT
                uuid,
                name,
                slug,
                template_type AS "template_type: EmailTemplateType",
                subject_template,
                body_html_template,
                body_text_template,
                variables,
                created_at,
                updated_at,
                created_by,
                updated_by
            FROM email_templates
            ORDER BY name ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(rows)
    }

    async fn list_by_type(&self, template_type: EmailTemplateType) -> Result<Vec<EmailTemplate>> {
        let rows = sqlx::query_as!(
            EmailTemplate,
            r#"
            SELECT
                uuid,
                name,
                slug,
                template_type AS "template_type: EmailTemplateType",
                subject_template,
                body_html_template,
                body_text_template,
                variables,
                created_at,
                updated_at,
                created_by,
                updated_by
            FROM email_templates
            WHERE template_type = $1
            ORDER BY name ASC
            "#,
            template_type as EmailTemplateType
        )
        .fetch_all(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(rows)
    }

    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<EmailTemplate>> {
        let row = sqlx::query_as!(
            EmailTemplate,
            r#"
            SELECT
                uuid,
                name,
                slug,
                template_type AS "template_type: EmailTemplateType",
                subject_template,
                body_html_template,
                body_text_template,
                variables,
                created_at,
                updated_at,
                created_by,
                updated_by
            FROM email_templates
            WHERE uuid = $1
            "#,
            uuid
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(row)
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<EmailTemplate>> {
        let row = sqlx::query_as!(
            EmailTemplate,
            r#"
            SELECT
                uuid,
                name,
                slug,
                template_type AS "template_type: EmailTemplateType",
                subject_template,
                body_html_template,
                body_text_template,
                variables,
                created_at,
                updated_at,
                created_by,
                updated_by
            FROM email_templates
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(row)
    }

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
    ) -> Result<Uuid> {
        let uuid = sqlx::query_scalar!(
            r#"
            INSERT INTO email_templates (
                name, slug, template_type, subject_template,
                body_html_template, body_text_template, variables, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING uuid
            "#,
            name,
            slug,
            template_type as EmailTemplateType,
            subject,
            body_html,
            body_text,
            variables,
            created_by
        )
        .fetch_one(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(uuid)
    }

    async fn update(
        &self,
        uuid: Uuid,
        name: Option<&str>,
        subject: &str,
        body_html: &str,
        body_text: &str,
        variables: serde_json::Value,
        updated_by: Uuid,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE email_templates
            SET
                name               = COALESCE($2, name),
                subject_template   = $3,
                body_html_template = $4,
                body_text_template = $5,
                variables          = $6,
                updated_by         = $7,
                updated_at         = NOW()
            WHERE uuid = $1
            "#,
            uuid,
            name,
            subject,
            body_html,
            body_text,
            variables,
            updated_by
        )
        .execute(&self.pool)
        .await
        .map_err(r_data_core_core::error::Error::Database)?;

        Ok(())
    }

    async fn delete(&self, uuid: Uuid) -> Result<()> {
        sqlx::query!("DELETE FROM email_templates WHERE uuid = $1", uuid)
            .execute(&self.pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        Ok(())
    }
}
