#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use super::transform::StringOperand;

/// Actions to execute after all items in a workflow run have been processed.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct OnComplete {
    pub actions: Vec<PostRunAction>,
}

/// A single post-run action.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PostRunAction {
    /// Send an email using a template and the run context.
    SendEmail(PostRunSendEmail),
}

/// Send an email after the run completes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct PostRunSendEmail {
    /// UUID of a workflow email template
    pub template_uuid: String,
    /// Recipients (only `const_string` — no field refs in post-run context)
    pub to: Vec<StringOperand>,
    /// Optional CC
    pub cc: Option<Vec<StringOperand>>,
    /// When to fire this action
    #[serde(default)]
    pub condition: PostRunCondition,
}

/// Condition for when a post-run action fires.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS, Default, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PostRunCondition {
    /// Fire after every run
    #[default]
    Always,
    /// Fire only when all items succeeded (`failed_items` == 0)
    OnSuccess,
    /// Fire only when at least one item failed
    OnFailure,
}

/// Validate the `on_complete` section of a workflow DSL program.
///
/// # Errors
/// Returns an error if any action has invalid fields.
pub fn validate_on_complete(
    on_complete: &OnComplete,
    safe_field: &regex::Regex,
) -> r_data_core_core::error::Result<()> {
    for (idx, action) in on_complete.actions.iter().enumerate() {
        match action {
            PostRunAction::SendEmail(email) => {
                validate_post_run_send_email(idx, email, safe_field)?;
            }
        }
    }
    Ok(())
}

fn validate_post_run_send_email(
    idx: usize,
    email: &PostRunSendEmail,
    safe_field: &regex::Regex,
) -> r_data_core_core::error::Result<()> {
    if email.template_uuid.trim().is_empty() {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "on_complete.actions[{idx}]: template_uuid must not be empty"
        )));
    }
    if email.to.is_empty() {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "on_complete.actions[{idx}]: to must not be empty"
        )));
    }
    for (i, op) in email.to.iter().enumerate() {
        if let StringOperand::Field { field } = op {
            if !safe_field.is_match(field) {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "on_complete.actions[{idx}].to[{i}]: field must be safe"
                )));
            }
        }
    }
    if let Some(ref cc) = email.cc {
        for (i, op) in cc.iter().enumerate() {
            if let StringOperand::Field { field } = op {
                if !safe_field.is_match(field) {
                    return Err(r_data_core_core::error::Error::Validation(format!(
                        "on_complete.actions[{idx}].cc[{i}]: field must be safe"
                    )));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn safe_field() -> Regex {
        Regex::new(r"^[A-Za-z_][A-Za-z0-9_.]*$").unwrap()
    }

    #[test]
    fn valid_on_complete_const_recipients() {
        let oc = OnComplete {
            actions: vec![PostRunAction::SendEmail(PostRunSendEmail {
                template_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
                to: vec![StringOperand::ConstString {
                    value: "admin@example.com".to_string(),
                }],
                cc: None,
                condition: PostRunCondition::Always,
            })],
        };
        assert!(validate_on_complete(&oc, &safe_field()).is_ok());
    }

    #[test]
    fn empty_template_uuid_fails() {
        let oc = OnComplete {
            actions: vec![PostRunAction::SendEmail(PostRunSendEmail {
                template_uuid: "  ".to_string(),
                to: vec![StringOperand::ConstString {
                    value: "a@b.com".to_string(),
                }],
                cc: None,
                condition: PostRunCondition::default(),
            })],
        };
        assert!(validate_on_complete(&oc, &safe_field()).is_err());
    }

    #[test]
    fn empty_to_fails() {
        let oc = OnComplete {
            actions: vec![PostRunAction::SendEmail(PostRunSendEmail {
                template_uuid: "uuid".to_string(),
                to: vec![],
                cc: None,
                condition: PostRunCondition::OnSuccess,
            })],
        };
        assert!(validate_on_complete(&oc, &safe_field()).is_err());
    }

    #[test]
    fn unsafe_field_in_to_fails() {
        let oc = OnComplete {
            actions: vec![PostRunAction::SendEmail(PostRunSendEmail {
                template_uuid: "uuid".to_string(),
                to: vec![StringOperand::Field {
                    field: "bad field!".to_string(),
                }],
                cc: None,
                condition: PostRunCondition::OnFailure,
            })],
        };
        assert!(validate_on_complete(&oc, &safe_field()).is_err());
    }

    #[test]
    fn unsafe_field_in_cc_fails() {
        let oc = OnComplete {
            actions: vec![PostRunAction::SendEmail(PostRunSendEmail {
                template_uuid: "uuid".to_string(),
                to: vec![StringOperand::ConstString {
                    value: "a@b.com".to_string(),
                }],
                cc: Some(vec![StringOperand::Field {
                    field: "bad!".to_string(),
                }]),
                condition: PostRunCondition::Always,
            })],
        };
        assert!(validate_on_complete(&oc, &safe_field()).is_err());
    }

    #[test]
    fn condition_defaults_to_always() {
        let json = r#"{"template_uuid":"uuid","to":[{"kind":"const_string","value":"a@b.com"}]}"#;
        let email: PostRunSendEmail = serde_json::from_str(json).unwrap();
        assert_eq!(email.condition, PostRunCondition::Always);
    }
}
