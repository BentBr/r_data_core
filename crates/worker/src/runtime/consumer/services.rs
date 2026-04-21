#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use r_data_core_persistence::{
    DynamicEntityRepository, EntityDefinitionRepository, SystemLogRepository, WorkflowRepository,
};
use r_data_core_services::adapters::{
    DynamicEntityRepositoryAdapter, EntityDefinitionRepositoryAdapter,
};
use r_data_core_services::{
    DynamicEntityService, EntityDefinitionService, SystemLogService, WorkflowRepositoryAdapter,
    WorkflowService,
};
use r_data_core_workflow::data::job_queue::JobQueue;

use super::state::ConsumerState;

pub(super) fn build_fetch_service(state: &ConsumerState) -> WorkflowService {
    let adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(state.pool.clone()));
    let mut service = WorkflowService::new(Arc::new(adapter));
    if let Some(outbox_repo) = state.outbox_repo.clone() {
        service = service.with_outbox_repository(outbox_repo);
        if let Some(policy) = state.outbox_retry_policy {
            service = service.with_outbox_retry_policy(policy);
        }
    }
    service
}

pub(super) fn build_processing_service(state: &ConsumerState) -> WorkflowService {
    let wf_adapter = WorkflowRepositoryAdapter::new(WorkflowRepository::new(state.pool.clone()));
    let de_repo = DynamicEntityRepository::new(state.pool.clone());
    let de_adapter = DynamicEntityRepositoryAdapter::new(de_repo);
    let ed_repo = EntityDefinitionRepository::new(state.pool.clone());
    let ed_adapter = EntityDefinitionRepositoryAdapter::new(ed_repo);
    let ed_service =
        EntityDefinitionService::new(Arc::new(ed_adapter), state.cache_manager.clone());
    let de_service = DynamicEntityService::new(Arc::new(de_adapter), Arc::new(ed_service));
    let system_log_service = Arc::new(SystemLogService::new(Arc::new(SystemLogRepository::new(
        state.pool.clone(),
    ))));
    let queue: Arc<dyn JobQueue> = state.queue.clone();
    let mut service =
        WorkflowService::new_with_entities(Arc::new(wf_adapter), Arc::new(de_service))
            .with_jwt_config(state.jwt_secret.clone(), state.jwt_expiration)
            .with_mail_service(state.workflow_mail_service.clone())
            .with_queue(Some(queue))
            .with_system_log(system_log_service);
    if let Some(outbox_repo) = state.outbox_repo.clone() {
        service = service.with_outbox_repository(outbox_repo);
        if let Some(policy) = state.outbox_retry_policy {
            service = service.with_outbox_retry_policy(policy);
        }
    }
    service
}
