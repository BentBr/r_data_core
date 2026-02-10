#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use super::WorkflowRepository;
use crate::workflow_versioning_repository::WorkflowVersioningRepository;
use r_data_core_core::error::Result;
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core_workflow::data::{Workflow, WorkflowKind};
use std::str::FromStr;

impl WorkflowRepository {
    /// Get a workflow by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<Workflow>> {
        let row = sqlx::query(
            "
            SELECT uuid, name, description, kind::text, enabled, schedule_cron, config, versioning_disabled
            FROM workflows
            WHERE uuid = $1
            ",
        )
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await?;

        row.map_or_else(
            || Ok(None),
            |r| {
                let uuid: Uuid = r
                    .try_get(0)
                    .map_err(r_data_core_core::error::Error::Database)?;
                let name: String = r
                    .try_get(1)
                    .map_err(r_data_core_core::error::Error::Database)?;
                let description: Option<String> = r.try_get(2).ok();
                let kind_str: String = r.try_get(3).unwrap_or_else(|_| "consumer".to_string());
                let kind = WorkflowKind::from_str(&kind_str).unwrap_or(WorkflowKind::Consumer);
                let enabled: bool = r
                    .try_get::<Option<bool>, _>(4)
                    .unwrap_or(Some(true))
                    .unwrap_or(true);
                let schedule_cron: Option<String> = r.try_get(5).ok();
                let config: serde_json::Value =
                    r.try_get(6).unwrap_or_else(|_| serde_json::json!({}));
                let versioning_disabled: bool = r
                    .try_get::<Option<bool>, _>(7)
                    .unwrap_or(Some(true))
                    .unwrap_or(true);
                let wf = Workflow {
                    uuid,
                    name,
                    description,
                    kind,
                    enabled,
                    schedule_cron,
                    config,
                    versioning_disabled,
                };
                Ok(Some(wf))
            },
        )
    }

    /// Create a new workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn create(&self, req: &CreateWorkflowRequest, created_by: Uuid) -> Result<Uuid> {
        let row = sqlx::query(
            "
            INSERT INTO workflows (name, description, kind, enabled, schedule_cron, config, versioning_disabled, created_by)
            VALUES ($1, $2, $3::workflow_kind, $4, $5, $6, $7, $8)
            RETURNING uuid
            ",
        )
        .bind(&req.name)
        .bind(req.description.as_deref())
        .bind(&req.kind) // req.kind is already a String
        .bind(req.enabled)
        .bind(req.schedule_cron.as_deref())
        .bind(&req.config)
        .bind(req.versioning_disabled)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("uuid")?)
    }

    /// Update an existing workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn update(
        &self,
        uuid: Uuid,
        req: &UpdateWorkflowRequest,
        updated_by: Uuid,
    ) -> Result<()> {
        // Pre-update snapshot of current workflow row
        let versioning_repo = WorkflowVersioningRepository::new(self.pool.clone());
        versioning_repo
            .snapshot_pre_update(uuid)
            .await
            .map_err(|e| {
                r_data_core_core::error::Error::Unknown(format!("Failed to snapshot workflow: {e}"))
            })?;

        sqlx::query(
            "
            UPDATE workflows
            SET name = $2, description = $3, kind = $4::workflow_kind, enabled = $5,
                schedule_cron = $6, config = $7, versioning_disabled = $8, updated_by = $9, version = version + 1, updated_at = NOW()
            WHERE uuid = $1
            ",
        )
        .bind(uuid)
        .bind(&req.name)
        .bind(req.description.as_deref())
        .bind(&req.kind) // req.kind is already a String
        .bind(req.enabled)
        .bind(req.schedule_cron.as_deref())
        .bind(&req.config)
        .bind(req.versioning_disabled)
        .bind(updated_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn delete(&self, uuid: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflows WHERE uuid = $1")
            .bind(uuid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List all workflows
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn list_all(&self) -> Result<Vec<Workflow>> {
        let rows = sqlx::query(
            "
            SELECT uuid, name, description, kind::text, enabled, schedule_cron, config, versioning_disabled
            FROM workflows
            ORDER BY name
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let kind_str: String = r.try_get(3).unwrap_or_else(|_| "consumer".to_string());
            let kind = WorkflowKind::from_str(&kind_str).unwrap_or(WorkflowKind::Consumer);
            out.push(Workflow {
                uuid: r
                    .try_get(0)
                    .map_err(r_data_core_core::error::Error::Database)?,
                name: r
                    .try_get(1)
                    .map_err(r_data_core_core::error::Error::Database)?,
                description: r.try_get(2).ok(),
                kind,
                enabled: r
                    .try_get::<Option<bool>, _>(4)
                    .unwrap_or(Some(true))
                    .unwrap_or(true),
                schedule_cron: r.try_get(5).ok(),
                config: r.try_get(6).unwrap_or_else(|_| serde_json::json!({})),
                versioning_disabled: r
                    .try_get::<Option<bool>, _>(7)
                    .unwrap_or(Some(false))
                    .unwrap_or(false),
            });
        }
        Ok(out)
    }

    /// List workflows with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<Workflow>> {
        // Build ORDER BY clause - field is already validated and sanitized by route handler
        let order_by = sort_by.map_or_else(
            || "\"name\" ASC".to_string(),
            |field| {
                let quoted_field = format!("\"{}\"", field.replace('"', "\"\""));
                let order = sort_order
                    .as_ref()
                    .map(|o| o.to_uppercase())
                    .filter(|o| o == "ASC" || o == "DESC")
                    .unwrap_or_else(|| "ASC".to_string());
                format!("{quoted_field} {order}")
            },
        );

        // Build query with or without LIMIT
        let query = if limit == i64::MAX {
            format!(
                "
                SELECT uuid, name, description, kind::text, enabled, schedule_cron, config, versioning_disabled
                FROM workflows
                ORDER BY {order_by} OFFSET $1
                "
            )
        } else {
            format!(
                "
                SELECT uuid, name, description, kind::text, enabled, schedule_cron, config, versioning_disabled
                FROM workflows
                ORDER BY {order_by} LIMIT $1 OFFSET $2
                "
            )
        };

        let mut query_builder = sqlx::query(&query);
        if limit == i64::MAX {
            query_builder = query_builder.bind(offset);
        } else {
            query_builder = query_builder.bind(limit).bind(offset);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let uuid: Uuid = r
                .try_get(0)
                .map_err(r_data_core_core::error::Error::Database)?;
            let name: String = r
                .try_get(1)
                .map_err(r_data_core_core::error::Error::Database)?;
            let description: Option<String> = r.try_get(2).ok();
            let kind_str: String = r.try_get(3).unwrap_or_else(|_| "consumer".to_string());
            let kind = WorkflowKind::from_str(&kind_str).unwrap_or(WorkflowKind::Consumer);
            let enabled: bool = r
                .try_get::<Option<bool>, _>(4)
                .unwrap_or(Some(true))
                .unwrap_or(true);
            let schedule_cron: Option<String> = r.try_get(5).ok();
            let config: serde_json::Value = r.try_get(6).unwrap_or_else(|_| serde_json::json!({}));
            let versioning_disabled: bool = r
                .try_get::<Option<bool>, _>(7)
                .unwrap_or(Some(false))
                .unwrap_or(false);
            out.push(Workflow {
                uuid,
                name,
                description,
                kind,
                enabled,
                schedule_cron,
                config,
                versioning_disabled,
            });
        }
        Ok(out)
    }

    /// Count all workflows
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn count_all(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) AS cnt FROM workflows")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get::<i64, _>("cnt")?)
    }

    /// Check if a workflow config has from.api source type (accepts POST, cron disabled)
    /// or to.format.output.mode === 'api' (exports via GET, cron disabled)
    fn check_has_api_endpoint(config: &Value) -> bool {
        if let Some(steps) = config.get("steps").and_then(|v| v.as_array()) {
            for step in steps {
                // Check for from.api source type (accepts POST, cron disabled)
                if let Some(from) = step.get("from") {
                    if let Some(source) = from
                        .get("source")
                        .or_else(|| from.get("format").and_then(|f| f.get("source")))
                    {
                        if let Some(source_type) =
                            source.get("source_type").and_then(|v| v.as_str())
                        {
                            if source_type == "api" {
                                // from.api without endpoint field = accepts POST
                                if let Some(config_obj) =
                                    source.get("config").and_then(|v| v.as_object())
                                {
                                    if !config_obj.contains_key("endpoint") {
                                        return true;
                                    }
                                } else {
                                    // No config object or empty config = accepts POST
                                    return true;
                                }
                            }
                        }
                    }
                }

                // Check for to.format.output.mode === 'api' (exports via GET, cron disabled)
                if let Some(to) = step.get("to") {
                    if let Some(to_type) = to.get("type").and_then(|v| v.as_str()) {
                        if to_type == "format" {
                            if let Some(output) = to.get("output") {
                                // Check if output is a string "api" or object with mode: "api"
                                if output.as_str() == Some("api") {
                                    return true;
                                }
                                if let Some(output_obj) = output.as_object() {
                                    if output_obj.get("mode").and_then(|v| v.as_str())
                                        == Some("api")
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// List scheduled workflow consumers
    ///
    /// # Errors
    /// Returns an error if the database query fails
    ///
    /// # Panics
    /// Panics if database row data is invalid
    pub async fn list_scheduled_consumers(&self) -> Result<Vec<(Uuid, String)>> {
        // Fetch workflows with their config to check for from.api source type
        let rows = sqlx::query(
            "SELECT uuid, schedule_cron, config FROM workflows WHERE enabled = true AND kind = 'consumer'::workflow_kind AND schedule_cron IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await
        ?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let uuid: Uuid = r
                .try_get(0)
                .map_err(r_data_core_core::error::Error::Database)?;
            let cron: String = r
                .try_get::<Option<String>, _>(1)
                .ok()
                .flatten()
                .unwrap_or_default();
            let config: Value = r.try_get(2).unwrap_or_else(|_| serde_json::json!({}));

            // Exclude workflows with from.api source type (they accept POST, not cron)
            if !Self::check_has_api_endpoint(&config) {
                out.push((uuid, cron));
            }
        }
        Ok(out)
    }
}
