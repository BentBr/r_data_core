#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::Row;
use uuid::Uuid;

use super::WorkflowRepository;
use r_data_core_core::error::Result;

impl WorkflowRepository {
    /// Insert raw items for a workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn insert_raw_items(
        &self,
        _workflow_uuid: Uuid,
        run_uuid: Uuid,
        payloads: Vec<serde_json::Value>,
    ) -> Result<i64> {
        // Determine next sequence number for this run
        let start_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq_no), 0) FROM workflow_raw_items WHERE workflow_run_uuid = $1",
        )
        .bind(run_uuid)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let mut count: i64 = 0;
        for (idx, payload) in payloads.into_iter().enumerate() {
            let seq_no = start_seq + i64::try_from(idx).unwrap_or(0) + 1;
            sqlx::query(
                "
                INSERT INTO workflow_raw_items (workflow_run_uuid, seq_no, payload, status)
                VALUES ($1, $2, $3, 'queued')
                ",
            )
            .bind(run_uuid)
            .bind(seq_no)
            .bind(payload)
            .execute(&self.pool)
            .await?;
            count += 1;
        }
        Ok(count)
    }

    /// Count raw items for a workflow run
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn count_raw_items_for_run(&self, run_uuid: Uuid) -> Result<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS cnt FROM workflow_raw_items WHERE workflow_run_uuid = $1",
        )
        .bind(run_uuid)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.try_get::<i64, _>("cnt")?)
    }

    /// Mark raw items as processed for a workflow run
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn mark_raw_items_processed(&self, run_uuid: Uuid) -> Result<()> {
        sqlx::query("UPDATE workflow_raw_items SET status = 'processed' WHERE workflow_run_uuid = $1 AND status = 'queued'")
            .bind(run_uuid)
            .execute(&self.pool)
            .await
            ?;
        Ok(())
    }

    /// Fetch staged raw items for a workflow run
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn fetch_staged_raw_items(
        &self,
        run_uuid: Uuid,
        limit: i64,
    ) -> Result<Vec<(Uuid, serde_json::Value)>> {
        let rows = sqlx::query(
            "
            SELECT uuid, payload
            FROM workflow_raw_items
            WHERE workflow_run_uuid = $1 AND status = 'queued'
            ORDER BY seq_no ASC
            LIMIT $2
            ",
        )
        .bind(run_uuid)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let uuid: Uuid = r.try_get("uuid")?;
            let payload: serde_json::Value = r.try_get("payload")?;
            out.push((uuid, payload));
        }
        Ok(out)
    }

    /// Set the status of a raw item
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn set_raw_item_status(
        &self,
        item_uuid: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "
            UPDATE workflow_raw_items
            SET status = $2::data_raw_item_status, error = $3
            WHERE uuid = $1
            ",
        )
        .bind(item_uuid)
        .bind(status)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
