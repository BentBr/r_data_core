#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use sqlx::Type;
use time::OffsetDateTime;
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, TS, PartialEq, Eq)]
#[sqlx(type_name = "system_log_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SystemLogStatus {
    Success,
    Failed,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, TS, PartialEq, Eq)]
#[sqlx(type_name = "system_log_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SystemLogType {
    EmailSent,
    EntityCreated,
    EntityUpdated,
    EntityDeleted,
    AuthEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, ToSchema, TS, PartialEq, Eq)]
#[sqlx(type_name = "system_log_resource_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SystemLogResourceType {
    Email,
    AdminUser,
    Role,
    Workflow,
    EntityDefinition,
    EmailTemplate,
    ApiKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLog {
    pub uuid: Uuid,
    pub created_at: OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub status: SystemLogStatus,
    pub log_type: SystemLogType,
    pub resource_type: SystemLogResourceType,
    pub resource_uuid: Option<Uuid>,
    pub summary: String,
    pub details: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_log_status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&SystemLogStatus::Success).unwrap(),
            "\"success\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogStatus::Failed).unwrap(),
            "\"failed\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogStatus::Pending).unwrap(),
            "\"pending\""
        );
    }

    #[test]
    fn system_log_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&SystemLogType::EmailSent).unwrap(),
            "\"email_sent\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogType::EntityCreated).unwrap(),
            "\"entity_created\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogType::EntityUpdated).unwrap(),
            "\"entity_updated\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogType::EntityDeleted).unwrap(),
            "\"entity_deleted\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogType::AuthEvent).unwrap(),
            "\"auth_event\""
        );
    }

    #[test]
    fn system_log_resource_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&SystemLogResourceType::Email).unwrap(),
            "\"email\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogResourceType::AdminUser).unwrap(),
            "\"admin_user\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogResourceType::Role).unwrap(),
            "\"role\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogResourceType::Workflow).unwrap(),
            "\"workflow\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogResourceType::EntityDefinition).unwrap(),
            "\"entity_definition\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogResourceType::EmailTemplate).unwrap(),
            "\"email_template\""
        );
        assert_eq!(
            serde_json::to_string(&SystemLogResourceType::ApiKey).unwrap(),
            "\"api_key\""
        );
    }

    #[test]
    fn system_log_status_deserializes() {
        let status: SystemLogStatus = serde_json::from_str("\"success\"").unwrap();
        assert_eq!(status, SystemLogStatus::Success);
        let status: SystemLogStatus = serde_json::from_str("\"failed\"").unwrap();
        assert_eq!(status, SystemLogStatus::Failed);
    }

    #[test]
    fn system_log_type_deserializes() {
        let t: SystemLogType = serde_json::from_str("\"auth_event\"").unwrap();
        assert_eq!(t, SystemLogType::AuthEvent);
        let t: SystemLogType = serde_json::from_str("\"email_sent\"").unwrap();
        assert_eq!(t, SystemLogType::EmailSent);
    }

    #[test]
    fn system_log_resource_type_deserializes() {
        let rt: SystemLogResourceType = serde_json::from_str("\"admin_user\"").unwrap();
        assert_eq!(rt, SystemLogResourceType::AdminUser);
        let rt: SystemLogResourceType = serde_json::from_str("\"email_template\"").unwrap();
        assert_eq!(rt, SystemLogResourceType::EmailTemplate);
    }

    #[test]
    fn system_log_status_equality() {
        assert_eq!(SystemLogStatus::Success, SystemLogStatus::Success);
        assert_ne!(SystemLogStatus::Success, SystemLogStatus::Failed);
    }
}
