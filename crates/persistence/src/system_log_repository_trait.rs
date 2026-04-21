#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use r_data_core_core::error::Result;
use r_data_core_core::system_log::{
    SystemLog, SystemLogResourceType, SystemLogStatus, SystemLogType,
};
use time::OffsetDateTime;
use uuid::Uuid;

/// Filter criteria for querying system logs
#[derive(Debug, Default, Clone)]
pub struct SystemLogFilter {
    pub log_type: Option<SystemLogType>,
    pub resource_type: Option<SystemLogResourceType>,
    pub status: Option<SystemLogStatus>,
    pub resource_uuid: Option<Uuid>,
    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,
}

/// Trait for system log repository operations
#[async_trait]
pub trait SystemLogRepositoryTrait: Send + Sync {
    /// Insert a new system log entry
    ///
    /// # Errors
    /// Returns an error if the database insert fails
    #[allow(clippy::too_many_arguments)]
    async fn insert(
        &self,
        created_by: Option<Uuid>,
        status: SystemLogStatus,
        log_type: SystemLogType,
        resource_type: SystemLogResourceType,
        resource_uuid: Option<Uuid>,
        summary: &str,
        details: Option<serde_json::Value>,
    ) -> Result<Uuid>;

    /// Get a system log entry by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<SystemLog>>;

    /// List system logs with pagination and optional filters
    ///
    /// Returns a tuple of `(logs, total_count)`.
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
        filter: &SystemLogFilter,
    ) -> Result<(Vec<SystemLog>, i64)>;

    /// Delete system log entries older than the given number of days
    ///
    /// # Errors
    /// Returns an error if the database delete fails
    async fn delete_older_than_days(&self, days: i64) -> Result<u64>;
}
