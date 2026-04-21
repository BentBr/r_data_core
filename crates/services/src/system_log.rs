#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use r_data_core_core::system_log::{SystemLogResourceType, SystemLogStatus, SystemLogType};
use r_data_core_persistence::SystemLogRepositoryTrait;
use uuid::Uuid;

pub struct SystemLogService {
    repo: Arc<dyn SystemLogRepositoryTrait>,
}

impl SystemLogService {
    pub fn new(repo: Arc<dyn SystemLogRepositoryTrait>) -> Self {
        Self { repo }
    }

    /// Log an email sent/failed event. Best-effort — errors are logged to stdout.
    pub async fn log_email_sent(
        &self,
        created_by: Option<Uuid>,
        resource_uuid: Option<Uuid>,
        summary: &str,
        details: Option<serde_json::Value>,
        status: SystemLogStatus,
    ) {
        if let Err(e) = self
            .repo
            .insert(
                created_by,
                status,
                SystemLogType::EmailSent,
                SystemLogResourceType::Email,
                resource_uuid,
                summary,
                details,
            )
            .await
        {
            log::error!("Failed to write system log (email_sent): {e}");
        }
    }

    /// Log an entity created event.
    pub async fn log_entity_created(
        &self,
        actor: Option<Uuid>,
        resource_type: SystemLogResourceType,
        resource_uuid: Uuid,
        summary: &str,
        details: Option<serde_json::Value>,
    ) {
        if let Err(e) = self
            .repo
            .insert(
                actor,
                SystemLogStatus::Success,
                SystemLogType::EntityCreated,
                resource_type,
                Some(resource_uuid),
                summary,
                details,
            )
            .await
        {
            log::error!("Failed to write system log (entity_created): {e}");
        }
    }

    /// Log an entity updated event.
    pub async fn log_entity_updated(
        &self,
        actor: Option<Uuid>,
        resource_type: SystemLogResourceType,
        resource_uuid: Uuid,
        summary: &str,
        details: Option<serde_json::Value>,
    ) {
        if let Err(e) = self
            .repo
            .insert(
                actor,
                SystemLogStatus::Success,
                SystemLogType::EntityUpdated,
                resource_type,
                Some(resource_uuid),
                summary,
                details,
            )
            .await
        {
            log::error!("Failed to write system log (entity_updated): {e}");
        }
    }

    /// Log an entity deleted event.
    pub async fn log_entity_deleted(
        &self,
        actor: Option<Uuid>,
        resource_type: SystemLogResourceType,
        resource_uuid: Uuid,
        summary: &str,
        details: Option<serde_json::Value>,
    ) {
        if let Err(e) = self
            .repo
            .insert(
                actor,
                SystemLogStatus::Success,
                SystemLogType::EntityDeleted,
                resource_type,
                Some(resource_uuid),
                summary,
                details,
            )
            .await
        {
            log::error!("Failed to write system log (entity_deleted): {e}");
        }
    }

    /// Log an auth event (login, password reset, etc.).
    pub async fn log_auth_event(
        &self,
        actor: Option<Uuid>,
        resource_uuid: Option<Uuid>,
        summary: &str,
        details: Option<serde_json::Value>,
        status: SystemLogStatus,
    ) {
        if let Err(e) = self
            .repo
            .insert(
                actor,
                status,
                SystemLogType::AuthEvent,
                SystemLogResourceType::AdminUser,
                resource_uuid,
                summary,
                details,
            )
            .await
        {
            log::error!("Failed to write system log (auth_event): {e}");
        }
    }
}
