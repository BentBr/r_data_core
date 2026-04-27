#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod claim;
mod insert;
mod maintenance;
mod transitions;
mod types;

pub use types::{OutboxMessageRecord, OutboxRepository};
