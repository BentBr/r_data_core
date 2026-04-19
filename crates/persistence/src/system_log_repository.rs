#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::fmt::Write as _;

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::system_log_repository_trait::SystemLogRepositoryTrait;
use r_data_core_core::error::{Error, Result};
use r_data_core_core::system_log::{
    SystemLog, SystemLogResourceType, SystemLogStatus, SystemLogType,
};

/// Repository for system log operations
pub struct SystemLogRepository {
    pool: PgPool,
}

impl SystemLogRepository {
    /// Create a new system log repository
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Decode a `SystemLog` from a raw `sqlx::postgres::PgRow`
fn row_to_system_log(row: &sqlx::postgres::PgRow) -> std::result::Result<SystemLog, sqlx::Error> {
    Ok(SystemLog {
        uuid: row.try_get("uuid")?,
        created_at: row.try_get("created_at")?,
        created_by: row.try_get("created_by")?,
        status: row.try_get("status")?,
        log_type: row.try_get("log_type")?,
        resource_type: row.try_get("resource_type")?,
        resource_uuid: row.try_get("resource_uuid")?,
        summary: row.try_get("summary")?,
        details: row.try_get("details")?,
    })
}

#[async_trait]
impl SystemLogRepositoryTrait for SystemLogRepository {
    async fn insert(
        &self,
        created_by: Option<Uuid>,
        status: SystemLogStatus,
        log_type: SystemLogType,
        resource_type: SystemLogResourceType,
        resource_uuid: Option<Uuid>,
        summary: &str,
        details: Option<serde_json::Value>,
    ) -> Result<Uuid> {
        let uuid = sqlx::query_scalar!(
            r#"
            INSERT INTO system_logs (
                created_by, status, log_type, resource_type,
                resource_uuid, summary, details
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING uuid
            "#,
            created_by,
            status as SystemLogStatus,
            log_type as SystemLogType,
            resource_type as SystemLogResourceType,
            resource_uuid,
            summary,
            details
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(uuid)
    }

    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<SystemLog>> {
        let row = sqlx::query_as!(
            SystemLog,
            r#"
            SELECT
                uuid,
                created_at,
                created_by,
                status      AS "status: SystemLogStatus",
                log_type    AS "log_type: SystemLogType",
                resource_type AS "resource_type: SystemLogResourceType",
                resource_uuid,
                summary,
                details
            FROM system_logs
            WHERE uuid = $1
            "#,
            uuid
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(row)
    }

    async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
        log_type_filter: Option<SystemLogType>,
        resource_type_filter: Option<SystemLogResourceType>,
        status_filter: Option<SystemLogStatus>,
    ) -> Result<(Vec<SystemLog>, i64)> {
        // Build a dynamic WHERE clause
        let mut conditions: Vec<String> = Vec::new();
        let mut param_index: i32 = 1;

        if log_type_filter.is_some() {
            conditions.push(format!("log_type = ${param_index}"));
            param_index += 1;
        }
        if resource_type_filter.is_some() {
            conditions.push(format!("resource_type = ${param_index}"));
            param_index += 1;
        }
        if status_filter.is_some() {
            conditions.push(format!("status = ${param_index}"));
            param_index += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            let mut s = String::from("WHERE ");
            let _ = write!(s, "{}", conditions.join(" AND "));
            s
        };

        let limit_param = param_index;
        param_index += 1;
        let offset_param = param_index;

        let data_query = format!(
            r"
            SELECT uuid, created_at, created_by, status, log_type, resource_type,
                   resource_uuid, summary, details
            FROM system_logs
            {where_clause}
            ORDER BY created_at DESC
            LIMIT ${limit_param} OFFSET ${offset_param}
            "
        );

        let count_query = format!("SELECT COUNT(*) FROM system_logs {where_clause}");

        // Bind parameters in the same order for both queries
        macro_rules! bind_filters {
            ($q:expr) => {{
                let mut q = $q;
                if let Some(ref v) = log_type_filter {
                    q = q.bind(v.clone());
                }
                if let Some(ref v) = resource_type_filter {
                    q = q.bind(v.clone());
                }
                if let Some(ref v) = status_filter {
                    q = q.bind(v.clone());
                }
                q
            }};
        }

        // Fetch total count
        let count_row = bind_filters!(sqlx::query(&count_query))
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)?;
        let total: i64 = count_row.try_get(0).map_err(Error::Database)?;

        // Fetch data rows
        let rows = bind_filters!(sqlx::query(&data_query))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(Error::Database)?;

        let logs = rows
            .iter()
            .map(row_to_system_log)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::Database)?;

        Ok((logs, total))
    }

    async fn delete_older_than_days(&self, days: i64) -> Result<u64> {
        let result = sqlx::query!(
            "DELETE FROM system_logs WHERE created_at < NOW() - make_interval(days => $1::int)",
            i32::try_from(days).unwrap_or(i32::MAX)
        )
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(result.rows_affected())
    }
}
