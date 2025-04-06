// Basic notification module - to be expanded in future
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::error::Result;

/// Notification priority
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationPriority {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Urgent priority
    Urgent,
}

/// Notification type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationType {
    /// Email notification
    Email,
    /// In-app notification
    InApp,
    /// SMS notification
    SMS,
    /// Push notification
    Push,
}

/// Notification status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationStatus {
    /// Notification is pending
    Pending,
    /// Notification is being sent
    Sending,
    /// Notification has been sent
    Sent,
    /// Notification has been read
    Read,
    /// Notification failed to send
    Failed,
    /// Notification has been cancelled
    Cancelled,
}

/// Notification model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// ID of the notification
    pub id: Uuid,
    /// Type of notification
    pub notification_type: NotificationType,
    /// Status of the notification
    pub status: NotificationStatus,
    /// Subject of the notification
    pub subject: String,
    /// Body of the notification
    pub body: String,
    /// ID of the recipient user
    pub recipient_id: Option<Uuid>,
    /// Email of the recipient
    pub recipient_email: Option<String>,
    /// Phone number of the recipient
    pub recipient_phone: Option<String>,
    /// Related entity ID
    pub related_entity_id: Option<String>,
    /// URL to action
    pub action_url: Option<String>,
    /// Priority of the notification
    pub priority: NotificationPriority,
    /// Scheduled delivery time
    pub scheduled_for: Option<DateTime<Utc>>,
    /// When the notification was sent
    pub sent_at: Option<DateTime<Utc>>,
    /// When the notification was read
    pub read_at: Option<DateTime<Utc>>,
    /// Number of retry attempts
    pub retry_count: i32,
    /// Error message if sending failed
    pub error_message: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Additional data
    pub additional_data: Option<serde_json::Value>,
}

/// Notification manager trait
pub trait NotificationManager {
    /// Send a notification
    fn send_notification(&self, notification: &Notification) -> Result<Notification>;
    
    /// Schedule a notification
    fn schedule_notification(&self, notification: &Notification, scheduled_for: DateTime<Utc>) -> Result<Notification>;
    
    /// Cancel a notification
    fn cancel_notification(&self, notification_id: Uuid) -> Result<Notification>;
    
    /// Mark a notification as read
    fn mark_as_read(&self, notification_id: Uuid) -> Result<Notification>;
    
    /// Get notifications for a user
    fn get_user_notifications(&self, user_id: Uuid, include_read: bool) -> Result<Vec<Notification>>;
} 