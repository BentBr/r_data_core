use r_data_core_persistence::WorkflowRepositoryTrait;
use std::sync::Arc;
use uuid::Uuid;

pub(super) struct WorkflowItemStatusHandler<'a> {
    run_uuid: Uuid,
    repo: &'a Arc<dyn WorkflowRepositoryTrait>,
}

impl<'a> WorkflowItemStatusHandler<'a> {
    pub(super) const fn new(run_uuid: Uuid, repo: &'a Arc<dyn WorkflowRepositoryTrait>) -> Self {
        Self { run_uuid, repo }
    }

    pub(super) async fn mark_item_processed(
        &self,
        item_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        if let Err(e) = self
            .repo
            .set_raw_item_status(item_uuid, "processed", None)
            .await
        {
            let db_meta = Self::extract_sqlx_meta(&e);
            log::error!("[workflow] Failed to mark item {item_uuid} as processed: {e}");
            self.log_status_update_error(item_uuid, "processed", &e, db_meta)
                .await;
            return Ok(false);
        }
        Ok(true)
    }

    pub(super) async fn mark_entity_operation_failed(
        &self,
        item_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        log::error!("[workflow] Item {item_uuid} failed: entity operation failed");
        if let Err(e) = self
            .repo
            .set_raw_item_status(item_uuid, "failed", Some("entity operation failed"))
            .await
        {
            log::error!("[workflow] Failed to mark item {item_uuid} as failed: {e}");
        }
        Ok(false)
    }

    pub(super) async fn handle_execution_error(
        &self,
        error: r_data_core_core::error::Error,
        item_uuid: Uuid,
    ) -> r_data_core_core::error::Result<bool> {
        let error_msg = error.to_string();
        log::error!("[workflow] Item {item_uuid} failed: {error_msg}");

        if let Err(set_err) = self
            .repo
            .set_raw_item_status(item_uuid, "failed", Some(&error_msg))
            .await
        {
            let db_meta = Self::extract_sqlx_meta(&set_err);
            log::error!("[workflow] Failed to mark item {item_uuid} as failed: {set_err}");
            self.log_status_update_error(item_uuid, "failed", &set_err, db_meta)
                .await;
        }

        if let Err(log_err) = self
            .repo
            .insert_run_log(
                self.run_uuid,
                "error",
                "Item processing failed",
                Some(serde_json::json!({
                    "item_uuid": item_uuid,
                    "error": error_msg,
                    "error_type": format!("{:?}", error)
                })),
            )
            .await
        {
            log::error!("[workflow] Failed to insert run log: {log_err}");
        }

        Ok(false)
    }

    async fn log_status_update_error(
        &self,
        item_uuid: Uuid,
        attempted_status: &str,
        error: &r_data_core_core::error::Error,
        db_meta: serde_json::Value,
    ) {
        if let Err(log_err) = self
            .repo
            .insert_run_log(
                self.run_uuid,
                "error",
                &format!("Failed to mark item {attempted_status}"),
                Some(serde_json::json!({
                    "item_uuid": item_uuid,
                    "attempted_status": attempted_status,
                    "error": error.to_string(),
                    "db": db_meta
                })),
            )
            .await
        {
            log::error!("[workflow] Failed to insert run log: {log_err}");
        }
    }

    fn extract_sqlx_meta(e: &r_data_core_core::error::Error) -> serde_json::Value {
        let (code, message) =
            if let r_data_core_core::error::Error::Database(sqlx::Error::Database(db_err)) = e {
                (
                    db_err.code().map(|s| s.to_string()),
                    Some(db_err.message().to_string()),
                )
            } else {
                let mut code: Option<String> = None;
                let mut message: Option<String> = None;
                let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(e);
                while let Some(err) = cause {
                    if let Some(sqlx::Error::Database(db_err)) = err.downcast_ref::<sqlx::Error>() {
                        code = db_err.code().map(|s| s.to_string());
                        message = Some(db_err.message().to_string());
                        break;
                    }
                    cause = err.source();
                }
                (code, message)
            };

        serde_json::json!({
            "code": code,
            "message": message,
            "chain": format!("{:?}", e),
        })
    }
}
