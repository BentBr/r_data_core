use anyhow::Context;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{Workflow, WorkflowKind};
use crate::api::admin::workflows::models::{CreateWorkflowRequest, UpdateWorkflowRequest};
use std::str::FromStr;
use super::repository_trait::WorkflowRepositoryTrait;

pub struct WorkflowRepository {
    pool: PgPool,
}

impl WorkflowRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_uuid(&self, uuid: Uuid) -> anyhow::Result<Option<Workflow>> {
        let row = sqlx::query(
            r#"
            SELECT uuid, name, description, kind::text, enabled, schedule_cron, consumer_config, provider_config
            FROM workflows
            WHERE uuid = $1
            "#,
        )
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await
        .context("get workflow by uuid")?;

        if let Some(r) = row {
            let kind_str: String = r.try_get(3).unwrap_or_else(|_| "consumer".to_string());
            let kind = WorkflowKind::from_str(&kind_str).unwrap_or(WorkflowKind::Consumer);
            let wf = Workflow {
                uuid: r.try_get(0).unwrap(),
                name: r.try_get(1).unwrap(),
                description: r.try_get(2).ok(),
                kind,
                enabled: r
                    .try_get::<Option<bool>, _>(4)
                    .unwrap_or(Some(true))
                    .unwrap_or(true),
                schedule_cron: r.try_get(5).ok(),
                consumer_config: r.try_get(6).ok(),
                provider_config: r.try_get(7).ok(),
            };
            Ok(Some(wf))
        } else {
            Ok(None)
        }
    }

    pub async fn create(&self, req: &CreateWorkflowRequest) -> anyhow::Result<Uuid> {
        let row = sqlx::query(
            r#"
            INSERT INTO workflows (name, description, kind, enabled, schedule_cron, consumer_config, provider_config, created_by)
            VALUES ($1, $2, $3::workflow_kind, $4, $5, $6, $7, uuid_generate_v7())
            RETURNING uuid
            "#,
        )
        .bind(&req.name)
        .bind(req.description.as_deref())
        .bind(req.kind.to_string())
        .bind(req.enabled)
        .bind(req.schedule_cron.as_deref())
        .bind(req.consumer_config.as_ref())
        .bind(req.provider_config.as_ref())
        .fetch_one(&self.pool)
        .await
        .context("insert workflows")?;

        Ok(row.try_get("uuid")?)
    }

    pub async fn update(&self, uuid: Uuid, req: &UpdateWorkflowRequest) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE workflows
            SET name = $2, description = $3, kind = $4::workflow_kind, enabled = $5,
                schedule_cron = $6, consumer_config = $7, provider_config = $8
            WHERE uuid = $1
            "#,
        )
        .bind(uuid)
        .bind(&req.name)
        .bind(req.description.as_deref())
        .bind(req.kind.to_string())
        .bind(req.enabled)
        .bind(req.schedule_cron.as_deref())
        .bind(req.consumer_config.as_ref())
        .bind(req.provider_config.as_ref())
        .execute(&self.pool)
        .await
        .context("update workflows")?;
        Ok(())
    }

    pub async fn delete(&self, uuid: Uuid) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM workflows WHERE uuid = $1")
            .bind(uuid)
            .execute(&self.pool)
            .await
            .context("delete workflows")?;
        Ok(())
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<Workflow>> {
        let rows = sqlx::query(
            r#"
            SELECT uuid, name, description, kind::text, enabled, schedule_cron, consumer_config, provider_config
            FROM workflows
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("query workflows")?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let kind_str: String = r.try_get(3).unwrap_or_else(|_| "consumer".to_string());
            let kind = WorkflowKind::from_str(&kind_str).unwrap_or(WorkflowKind::Consumer);
            out.push(Workflow {
                uuid: r.try_get(0).unwrap(),
                name: r.try_get(1).unwrap(),
                description: r.try_get(2).ok(),
                kind,
                enabled: r
                    .try_get::<Option<bool>, _>(4)
                    .unwrap_or(Some(true))
                    .unwrap_or(true),
                schedule_cron: r.try_get(5).ok(),
                consumer_config: r.try_get(6).ok(),
                provider_config: r.try_get(7).ok(),
            });
        }
        Ok(out)
    }

    pub async fn list_paginated(&self, limit: i64, offset: i64) -> anyhow::Result<Vec<Workflow>> {
        let rows = sqlx::query(
            r#"
            SELECT uuid, name, description, kind::text, enabled, schedule_cron, consumer_config, provider_config
            FROM workflows
            ORDER BY name
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("query workflows paginated")?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let kind_str: String = r.try_get(3).unwrap_or_else(|_| "consumer".to_string());
            let kind = WorkflowKind::from_str(&kind_str).unwrap_or(WorkflowKind::Consumer);
            out.push(Workflow {
                uuid: r.try_get(0).unwrap(),
                name: r.try_get(1).unwrap(),
                description: r.try_get(2).ok(),
                kind,
                enabled: r.try_get::<Option<bool>, _>(4).unwrap_or(Some(true)).unwrap_or(true),
                schedule_cron: r.try_get(5).ok(),
                consumer_config: r.try_get(6).ok(),
                provider_config: r.try_get(7).ok(),
            });
        }
        Ok(out)
    }

    pub async fn count_all(&self) -> anyhow::Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) AS cnt FROM workflows")
            .fetch_one(&self.pool)
            .await
            .context("count workflows")?;
        Ok(row.try_get::<i64, _>("cnt")?)
    }

    pub async fn list_scheduled_consumers(&self) -> anyhow::Result<Vec<(Uuid, String)>> {
        let rows = sqlx::query(
            r#"SELECT uuid, schedule_cron FROM workflows WHERE enabled = true AND kind = 'consumer'::workflow_kind AND schedule_cron IS NOT NULL"#,
        )
        .fetch_all(&self.pool)
        .await
        .context("query scheduled consumers")?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let uuid: Uuid = r.try_get(0).unwrap();
            let cron: String = r.try_get::<Option<String>, _>(1).unwrap().unwrap();
            out.push((uuid, cron));
        }
        Ok(out)
    }

    pub async fn insert_run_queued(
        &self,
        workflow_uuid: Uuid,
        trigger_id: Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO workflow_runs (workflow_uuid, status, trigger_id) VALUES ($1, 'queued', $2)"#,
        )
        .bind(workflow_uuid)
        .bind(trigger_id)
        .execute(&self.pool)
        .await
        .context("insert workflow run queued")?;
        Ok(())
    }

    pub async fn list_queued_runs(&self, limit: i64) -> anyhow::Result<Vec<Uuid>> {
        let rows = sqlx::query(r#"SELECT uuid FROM workflow_runs WHERE status = 'queued' ORDER BY queued_at ASC LIMIT $1"#)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .context("list queued runs")?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(r.try_get::<Uuid, _>("uuid")?);
        }
        Ok(out)
    }

    pub async fn mark_run_running(&self, run_uuid: Uuid) -> anyhow::Result<()> {
        sqlx::query(r#"UPDATE workflow_runs SET status = 'running', started_at = NOW() WHERE uuid = $1 AND status = 'queued'"#)
            .bind(run_uuid)
            .execute(&self.pool)
            .await
            .context("mark run running")?;
        Ok(())
    }

    pub async fn mark_run_success(&self, run_uuid: Uuid, processed: i64, failed: i64) -> anyhow::Result<()> {
        sqlx::query(r#"UPDATE workflow_runs SET status = 'success', finished_at = NOW(), processed_items = $2, failed_items = $3 WHERE uuid = $1"#)
            .bind(run_uuid)
            .bind(processed)
            .bind(failed)
            .execute(&self.pool)
            .await
            .context("mark run success")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl WorkflowRepositoryTrait for WorkflowRepository {
    async fn list_all(&self) -> anyhow::Result<Vec<Workflow>> { self.list_all().await }
    async fn list_paginated(&self, limit: i64, offset: i64) -> anyhow::Result<Vec<Workflow>> { self.list_paginated(limit, offset).await }
    async fn count_all(&self) -> anyhow::Result<i64> { self.count_all().await }
    async fn get_by_uuid(&self, uuid: Uuid) -> anyhow::Result<Option<Workflow>> { self.get_by_uuid(uuid).await }
    async fn create(&self, req: &CreateWorkflowRequest) -> anyhow::Result<Uuid> { self.create(req).await }
    async fn update(&self, uuid: Uuid, req: &UpdateWorkflowRequest) -> anyhow::Result<()> { self.update(uuid, req).await }
    async fn delete(&self, uuid: Uuid) -> anyhow::Result<()> { self.delete(uuid).await }
    async fn list_scheduled_consumers(&self) -> anyhow::Result<Vec<(Uuid, String)>> { self.list_scheduled_consumers().await }
    async fn insert_run_queued(&self, workflow_uuid: Uuid, trigger_id: Uuid) -> anyhow::Result<()> { self.insert_run_queued(workflow_uuid, trigger_id).await }
}
