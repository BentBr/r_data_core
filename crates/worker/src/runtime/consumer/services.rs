#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use r_data_core_persistence::{
    DynamicEntityRepository, EntityDefinitionRepository, WorkflowRepository,
};
use r_data_core_services::adapters::{
    DynamicEntityRepositoryAdapter, EntityDefinitionRepositoryAdapter,
};
use r_data_core_services::{
    DynamicEntityService, EntityDefinitionService, WorkflowRepositoryAdapter, WorkflowService,
};

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
    let mut service =
        WorkflowService::new_with_entities(Arc::new(wf_adapter), Arc::new(de_service))
            .with_jwt_config(state.jwt_secret.clone(), state.jwt_expiration);
    if let Some(outbox_repo) = state.outbox_repo.clone() {
        service = service.with_outbox_repository(outbox_repo);
        if let Some(policy) = state.outbox_retry_policy {
            service = service.with_outbox_retry_policy(policy);
        }
    }
    service
}
