#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Job for fetching and staging workflow data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchAndStageJob {
    /// Workflow UUID
    pub workflow_id: Uuid,
    /// Optional trigger UUID
    pub trigger_id: Option<Uuid>,
}

/// Job for processing a raw item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRawItemJob {
    /// Raw item UUID
    pub raw_item_id: Uuid,
}

/// Job for sending an email via the worker's email consumer loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailJob {
    /// Workflow run UUID for logging (None for system emails)
    pub run_uuid: Option<Uuid>,
    /// Recipient email addresses
    pub to: Vec<String>,
    /// CC email addresses
    pub cc: Vec<String>,
    /// Rendered email subject
    pub subject: String,
    /// Rendered plain text body
    pub body_text: String,
    /// Rendered HTML body (optional)
    pub body_html: Option<String>,
    /// Override the `from_name` (e.g., workflow name)
    pub from_name_override: Option<String>,
    /// "workflow" or "system" — selects which SMTP config to use
    pub source: String,
    /// Email template UUID for system log linking
    pub template_uuid: Option<Uuid>,
    /// Serialized JSON context for template rendering.
    /// When `template_uuid` is set the worker loads the template from DB and renders
    /// using this context instead of the pre-rendered `subject`/`body_text`/`body_html`.
    #[serde(default)]
    pub template_context: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_email_job_serializes_correctly() {
        let job = SendEmailJob {
            run_uuid: Some(Uuid::nil()),
            to: vec!["alice@example.com".to_string()],
            cc: vec![],
            subject: "Test Subject".to_string(),
            body_text: "Hello".to_string(),
            body_html: Some("<p>Hello</p>".to_string()),
            from_name_override: Some("Test Workflow".to_string()),
            source: "workflow".to_string(),
            template_uuid: None,
            template_context: Some("{\"name\":\"Alice\"}".to_string()),
        };
        let json = serde_json::to_string(&job).unwrap();
        let deserialized: SendEmailJob = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.to, vec!["alice@example.com"]);
        assert_eq!(deserialized.source, "workflow");
        assert_eq!(
            deserialized.from_name_override,
            Some("Test Workflow".to_string())
        );
        assert!(deserialized.template_context.is_some());
    }

    #[test]
    fn send_email_job_template_context_defaults_to_none() {
        // Backward compat: old jobs without template_context should deserialize
        let json = r#"{
            "run_uuid": null,
            "to": ["bob@example.com"],
            "cc": [],
            "subject": "Hi",
            "body_text": "Body",
            "body_html": null,
            "from_name_override": null,
            "source": "system",
            "template_uuid": null
        }"#;
        let job: SendEmailJob = serde_json::from_str(json).unwrap();
        assert!(job.template_context.is_none());
        assert_eq!(job.source, "system");
    }

    #[test]
    fn send_email_job_with_all_fields_populated() {
        let job = SendEmailJob {
            run_uuid: Some(Uuid::now_v7()),
            to: vec!["a@b.com".to_string(), "c@d.com".to_string()],
            cc: vec!["cc@test.com".to_string()],
            subject: "Subject".to_string(),
            body_text: "Text".to_string(),
            body_html: Some("<b>HTML</b>".to_string()),
            from_name_override: Some("My Workflow".to_string()),
            source: "workflow".to_string(),
            template_uuid: Some(Uuid::nil()),
            template_context: Some("{}".to_string()),
        };
        let roundtrip: SendEmailJob =
            serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        assert_eq!(roundtrip.to.len(), 2);
        assert_eq!(roundtrip.cc.len(), 1);
        assert!(roundtrip.template_uuid.is_some());
    }
}
