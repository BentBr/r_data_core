#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use sqlx::Type;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, PartialEq, Eq)]
#[sqlx(type_name = "email_template_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EmailTemplateType {
    System,
    Workflow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    pub uuid: Uuid,
    pub name: String,
    pub slug: String,
    pub template_type: EmailTemplateType,
    pub subject_template: String,
    pub body_html_template: String,
    pub body_text_template: String,
    pub variables: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_template_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&EmailTemplateType::System).unwrap(),
            "\"system\""
        );
        assert_eq!(
            serde_json::to_string(&EmailTemplateType::Workflow).unwrap(),
            "\"workflow\""
        );
    }

    #[test]
    fn email_template_type_deserializes() {
        let t: EmailTemplateType = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(t, EmailTemplateType::System);
        let t: EmailTemplateType = serde_json::from_str("\"workflow\"").unwrap();
        assert_eq!(t, EmailTemplateType::Workflow);
    }

    #[test]
    fn email_template_type_equality() {
        assert_eq!(EmailTemplateType::System, EmailTemplateType::System);
        assert_ne!(EmailTemplateType::System, EmailTemplateType::Workflow);
    }

    #[test]
    fn invalid_template_type_fails_to_deserialize() {
        let result: Result<EmailTemplateType, _> = serde_json::from_str("\"invalid\"");
        assert!(result.is_err());
    }
}
