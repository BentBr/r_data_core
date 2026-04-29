use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::outbox::PushDispatchMode;
use crate::workflow::transform_execution::{JwtConfig, MailContext};
use r_data_core_persistence::WorkflowRepositoryTrait;
use std::sync::Arc;

/// Shared context for workflow item processing
pub struct WorkflowItemContext<'a> {
    pub dynamic_entity_service: Option<&'a DynamicEntityService>,
    pub push_dispatch: PushDispatchMode<'a>,
    pub repo: &'a Arc<dyn WorkflowRepositoryTrait>,
    pub jwt: &'a JwtConfig<'a>,
    pub mail: &'a MailContext<'a>,
    pub workflow_name: Option<&'a str>,
    pub versioning_disabled: bool,
}
