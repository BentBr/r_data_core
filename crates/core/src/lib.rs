#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod admin_jwt;
pub mod admin_user;
pub mod cache;
pub mod config;
pub mod crypto;
pub mod domain;
pub mod email_template;
pub mod entity_definition;
pub mod entity_jwt;
pub mod error;
pub mod field;
pub mod maintenance;
pub mod outbox;
pub mod password_reset_token;
pub mod permissions;
pub mod public_api;
pub mod refresh_token;
pub mod settings;
pub mod system_log;
pub mod utils;
pub mod validation;
pub mod versioning;

// Re-export DynamicEntity from domain
pub use domain::dynamic_entity::DynamicEntity;
pub use outbox::{
    OutboxMessage, OutboxStatus, WORKFLOW_FETCH_ENQUEUE_KIND, WORKFLOW_FETCH_TOPIC,
    WORKFLOW_PUSH_ENQUEUE_KIND, WORKFLOW_PUSH_TOPIC,
};
