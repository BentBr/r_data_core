use cron::Schedule;
use std::str::FromStr;
use uuid::Uuid;

use r_data_core_core::system_log::SystemLogResourceType;
use r_data_core_workflow::data::requests::{CreateWorkflowRequest, UpdateWorkflowRequest};
use r_data_core_workflow::data::Workflow;

use super::WorkflowService;

impl WorkflowService {
    /// List all workflows
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list(&self) -> r_data_core_core::error::Result<Vec<Workflow>> {
        self.repo.list_all().await
    }

    /// Get a workflow by UUID
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn get(&self, uuid: Uuid) -> r_data_core_core::error::Result<Option<Workflow>> {
        let key = Self::cache_key_by_uuid(&uuid);

        if let Some(cache) = &self.cache_manager {
            if let Ok(Some(cached)) = cache.get::<Workflow>(&key).await {
                return Ok(Some(cached));
            }
        }

        let workflow = self.repo.get_by_uuid(uuid).await?;

        if let (Some(cache), Some(found)) = (&self.cache_manager, workflow.as_ref()) {
            // Explicit short TTL, never `None`: see WORKFLOW_CACHE_TTL_SECS. A
            // `None` here would inherit the 3600s default and let a revoked
            // pre-shared key keep working in another process for an hour.
            // Cache errors are not fatal: a miss just costs a DB read.
            if let Err(e) = cache
                .set(&key, found, Some(super::cache::WORKFLOW_CACHE_TTL_SECS))
                .await
            {
                log::warn!("Failed to cache workflow {uuid}: {e}");
            }
        }

        Ok(workflow)
    }

    /// Create a new workflow
    ///
    /// # Errors
    /// Returns an error if validation fails or database operation fails
    pub async fn create(
        &self,
        req: &CreateWorkflowRequest,
        created_by: Uuid,
    ) -> r_data_core_core::error::Result<Uuid> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!("Invalid cron schedule: {e}"))
            })?;
        }
        // Strict DSL: parse and validate
        let program =
            r_data_core_workflow::dsl::DslProgram::from_config(&req.config).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!(
                    "Invalid workflow DSL configuration: {e}"
                ))
            })?;
        program.validate().map_err(|e| {
            r_data_core_core::error::Error::Validation(format!(
                "Workflow DSL validation failed: {e}"
            ))
        })?;

        // `provider_auth` sits alongside the DSL, not inside it, so the parse
        // above never sees it. It is the credential guarding the public
        // endpoint, so it gets its own check.
        r_data_core_workflow::data::validate_provider_auth_config(&req.config)?;
        let uuid = self.repo.create(req, created_by).await?;

        if let Some(ref log) = self.system_log {
            log.log_entity_created(
                Some(created_by),
                SystemLogResourceType::Workflow,
                uuid,
                &format!("Workflow '{}' created", req.name),
                Some(serde_json::json!({"name": req.name})),
            )
            .await;
        }

        Ok(uuid)
    }

    /// Update an existing workflow
    ///
    /// # Errors
    /// Returns an error if validation fails or database operation fails
    pub async fn update(
        &self,
        uuid: Uuid,
        req: &UpdateWorkflowRequest,
        updated_by: Uuid,
    ) -> r_data_core_core::error::Result<()> {
        if let Some(expr) = &req.schedule_cron {
            Schedule::from_str(expr).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!("Invalid cron schedule: {e}"))
            })?;
        }
        // Strict DSL: parse and validate
        let program =
            r_data_core_workflow::dsl::DslProgram::from_config(&req.config).map_err(|e| {
                r_data_core_core::error::Error::Validation(format!(
                    "Invalid workflow DSL configuration: {e}"
                ))
            })?;
        program.validate().map_err(|e| {
            r_data_core_core::error::Error::Validation(format!(
                "Workflow DSL validation failed: {e}"
            ))
        })?;

        // `provider_auth` sits alongside the DSL, not inside it, so the parse
        // above never sees it. It is the credential guarding the public
        // endpoint, so it gets its own check.
        r_data_core_workflow::data::validate_provider_auth_config(&req.config)?;
        self.repo.update(uuid, req, updated_by).await?;
        self.invalidate_workflow_cache(&uuid).await;

        if let Some(ref log) = self.system_log {
            log.log_entity_updated(
                Some(updated_by),
                SystemLogResourceType::Workflow,
                uuid,
                &format!("Workflow '{}' updated", req.name),
                Some(serde_json::json!({"name": req.name})),
            )
            .await;
        }

        Ok(())
    }

    /// Delete a workflow
    ///
    /// # Errors
    /// Returns an error if the database operation fails
    pub async fn delete(
        &self,
        uuid: Uuid,
        actor_uuid: Uuid,
    ) -> r_data_core_core::error::Result<()> {
        // Capture the workflow name before deletion for audit log
        let workflow_name = self
            .repo
            .get_by_uuid(uuid)
            .await
            .ok()
            .flatten()
            .map_or_else(|| uuid.to_string(), |w| w.name);

        self.repo.delete(uuid).await?;
        self.invalidate_workflow_cache(&uuid).await;

        if let Some(ref log) = self.system_log {
            log.log_entity_deleted(
                Some(actor_uuid),
                SystemLogResourceType::Workflow,
                uuid,
                &format!("Workflow '{workflow_name}' deleted"),
                Some(serde_json::json!({"name": workflow_name})),
            )
            .await;
        }

        Ok(())
    }

    /// List workflows with pagination
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_paginated(
        &self,
        limit: i64,
        offset: i64,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> r_data_core_core::error::Result<(Vec<Workflow>, i64)> {
        let (items, total) = tokio::try_join!(
            self.repo.list_paginated(limit, offset, sort_by, sort_order),
            self.repo.count_all()
        )?;
        Ok((items, total))
    }

    /// List workflows with query validation
    ///
    /// This method validates the query parameters and returns validated parameters along with workflows.
    ///
    /// # Arguments
    /// * `params` - The query parameters
    /// * `field_validator` - The `FieldValidator` instance (required for validation)
    ///
    /// # Returns
    /// A tuple of ((workflows, total), `validated_query`) where `validated_query` contains pagination metadata
    ///
    /// # Errors
    /// Returns an error if validation fails or database query fails
    pub async fn list_paginated_with_query(
        &self,
        params: &crate::query_validation::ListQueryParams,
        field_validator: &crate::query_validation::FieldValidator,
    ) -> r_data_core_core::error::Result<(
        (Vec<Workflow>, i64),
        crate::query_validation::ValidatedListQuery,
    )> {
        use crate::query_validation::validate_list_query;

        let validated =
            validate_list_query(params, "workflows", field_validator, 20, 100, true, &[])
                .await
                .map_err(|e| {
                    r_data_core_core::error::Error::Validation(format!(
                        "Query validation failed: {e}"
                    ))
                })?;

        let (items, total) = tokio::try_join!(
            self.repo.list_paginated(
                validated.limit,
                validated.offset,
                validated.sort_by.clone(),
                validated.sort_order.clone(),
            ),
            self.repo.count_all()
        )?;

        Ok(((items, total), validated))
    }
}
