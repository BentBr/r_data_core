use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::AbstractRDataEntity;

/// Types of notifications that can be sent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum NotificationType {
    /// In-app notification
    System,

    /// Email notification
    Email,

    /// SMS notification
    SMS,

    /// Push notification
    Push,

    /// Webhook notification
    Webhook,

    /// Slack notification
    Slack,

    /// Custom notification type
    Custom(String),
}

/// Status of a notification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum NotificationStatus {
    /// Not yet sent
    Pending,

    /// Being processed
    Processing,

    /// Successfully sent
    Sent,

    /// Read by recipient
    Read,

    /// Failed to send
    Failed,

    /// Canceled before sending
    Canceled,
}

/// Notification priority level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Ord, PartialOrd, Eq, ToSchema)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Entity for representing a notification
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Notification {
    /// Base entity properties
    #[serde(flatten)]
    pub base: AbstractRDataEntity,

    /// Type of notification
    pub notification_type: NotificationType,

    /// Current status
    pub status: NotificationStatus,

    /// Notification subject/title
    pub subject: String,

    /// Notification body content
    pub body: String,

    /// Recipient user ID (None for broadcast)
    pub recipient_id: Option<Uuid>,

    /// Recipient email (for email notifications)
    pub recipient_email: Option<String>,

    /// Recipient phone (for SMS notifications)
    pub recipient_phone: Option<String>,

    /// ID of entity this notification relates to
    pub related_entity_id: Option<String>,

    /// URL to link to from notification
    pub action_url: Option<String>,

    /// Priority level
    pub priority: NotificationPriority,

    /// When to send the notification (if scheduled)
    pub scheduled_for: Option<DateTime<Utc>>,

    /// When the notification was sent
    pub sent_at: Option<DateTime<Utc>>,

    /// When the notification was read
    pub read_at: Option<DateTime<Utc>>,

    /// Retry count if sending failed
    pub retry_count: i32,

    /// Error message if sending failed
    pub error_message: Option<String>,

    /// Additional data for the notification
    pub additional_data: Option<serde_json::Value>,
}

impl Notification {
    /// Create a new notification
    pub fn new(notification_type: NotificationType, subject: String, body: String) -> Self {
        Self {
            base: AbstractRDataEntity::new("/notifications".to_string()),
            notification_type,
            status: NotificationStatus::Pending,
            subject,
            body,
            recipient_id: None,
            recipient_email: None,
            recipient_phone: None,
            related_entity_id: None,
            action_url: None,
            priority: NotificationPriority::Normal,
            scheduled_for: None,
            sent_at: None,
            read_at: None,
            retry_count: 0,
            error_message: None,
            additional_data: None,
        }
    }

    /// Set the recipient user ID
    pub fn with_recipient_id(mut self, recipient_id: Uuid) -> Self {
        self.recipient_id = Some(recipient_id);
        self
    }

    /// Set the recipient email
    pub fn with_recipient_email(mut self, email: String) -> Self {
        self.recipient_email = Some(email);
        self
    }

    /// Set the notification priority
    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Schedule the notification for a future time
    pub fn schedule_for(mut self, time: DateTime<Utc>) -> Self {
        self.scheduled_for = Some(time);
        self
    }

    /// Mark the notification as sent
    pub fn mark_as_sent(&mut self) {
        self.status = NotificationStatus::Sent;
        self.sent_at = Some(Utc::now());
    }

    /// Mark the notification as read
    pub fn mark_as_read(&mut self) {
        self.status = NotificationStatus::Read;
        self.read_at = Some(Utc::now());
    }

    /// Mark the notification as failed
    pub fn mark_as_failed(&mut self, error: &str) {
        self.status = NotificationStatus::Failed;
        self.error_message = Some(error.to_string());
        self.retry_count += 1;
    }

    /// Check if the notification is ready to be sent
    pub fn is_ready_to_send(&self) -> bool {
        if let NotificationStatus::Pending = self.status {
            if let Some(scheduled_time) = self.scheduled_for {
                Utc::now() >= scheduled_time
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Validate the notification data is complete based on type
    pub fn validate(&self) -> Result<()> {
        match self.notification_type {
            NotificationType::Email => {
                if self.recipient_email.is_none() && self.recipient_id.is_none() {
                    return Err(Error::Validation(
                        "Email notification requires either recipient_email or recipient_id"
                            .to_string(),
                    ));
                }
            }
            NotificationType::SMS => {
                if self.recipient_phone.is_none() && self.recipient_id.is_none() {
                    return Err(Error::Validation(
                        "SMS notification requires either recipient_phone or recipient_id"
                            .to_string(),
                    ));
                }
            }
            NotificationType::System => {
                if self.recipient_id.is_none() {
                    return Err(Error::Validation(
                        "System notification requires recipient_id".to_string(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }
}
