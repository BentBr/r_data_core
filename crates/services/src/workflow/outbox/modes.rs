#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_persistence::OutboxRepositoryTrait;

use super::OutboxRetryPolicy;

#[derive(Clone, Copy)]
pub enum FetchDispatchMode<'a> {
    Direct,
    Outbox {
        repository: &'a dyn OutboxRepositoryTrait,
        retry_policy: Option<&'a OutboxRetryPolicy>,
    },
}

#[derive(Clone, Copy)]
pub enum PushDispatchMode<'a> {
    Direct,
    Outbox {
        repository: &'a dyn OutboxRepositoryTrait,
    },
}
