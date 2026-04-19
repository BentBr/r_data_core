#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use r_data_core_core::email_template::{EmailTemplate, EmailTemplateType};

/// Query parameters for filtering email templates
#[derive(Debug, Deserialize, ToSchema)]
pub struct EmailTemplateListQuery {
    /// Filter by template type: "system" or "workflow"
    #[serde(rename = "type")]
    pub template_type: Option<EmailTemplateType>,
}

/// Request body for creating a new email template
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEmailTemplateRequest {
    /// Display name for the template
    pub name: String,
    /// Unique slug identifier
    pub slug: String,
    /// Subject line (may contain template variables)
    pub subject_template: String,
    /// HTML body (may contain template variables)
    pub body_html_template: String,
    /// Plain-text body (may contain template variables)
    pub body_text_template: String,
    /// JSON object describing available template variables
    pub variables: serde_json::Value,
}

/// Request body for updating an email template
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateEmailTemplateRequest {
    /// New display name (only honoured for workflow templates)
    pub name: Option<String>,
    /// Updated subject line
    pub subject_template: String,
    /// Updated HTML body
    pub body_html_template: String,
    /// Updated plain-text body
    pub body_text_template: String,
    /// Updated variables schema
    pub variables: serde_json::Value,
}

/// Email template response DTO
#[derive(Debug, Serialize, ToSchema)]
pub struct EmailTemplateResponse {
    /// Template UUID
    pub uuid: Uuid,
    /// Display name
    pub name: String,
    /// Unique slug identifier
    pub slug: String,
    /// Template type (system or workflow)
    pub template_type: EmailTemplateType,
    /// Subject line template
    pub subject_template: String,
    /// HTML body template
    pub body_html_template: String,
    /// Plain-text body template
    pub body_text_template: String,
    /// Available template variables
    pub variables: serde_json::Value,
    /// ISO 8601 creation timestamp
    pub created_at: String,
    /// ISO 8601 last-updated timestamp
    pub updated_at: String,
}

impl From<EmailTemplate> for EmailTemplateResponse {
    fn from(t: EmailTemplate) -> Self {
        use time::format_description::well_known::Rfc3339;
        Self {
            uuid: t.uuid,
            name: t.name,
            slug: t.slug,
            template_type: t.template_type,
            subject_template: t.subject_template,
            body_html_template: t.body_html_template,
            body_text_template: t.body_text_template,
            variables: t.variables,
            created_at: t
                .created_at
                .format(&Rfc3339)
                .unwrap_or_else(|_| t.created_at.to_string()),
            updated_at: t
                .updated_at
                .format(&Rfc3339)
                .unwrap_or_else(|_| t.updated_at.to_string()),
        }
    }
}
