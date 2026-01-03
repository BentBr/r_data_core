#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod consumer_loop;
pub mod database;
pub mod entities;
pub mod queue;

// Re-export commonly used items
pub use consumer_loop::{
    create_test_queue, spawn_test_consumer_loop, ConsumerLoopConfig, ConsumerLoopHandle,
};
pub use database::{
    cleanup_orphaned_test_schemas, clear_entity_definitions, clear_refresh_tokens, clear_test_db,
    fast_clear_test_db, random_string, setup_test_db, teardown_test_schema, unique_entity_type,
    TestDatabase,
};
pub use entities::{
    create_entity_definition_from_json, create_test_admin_user, create_test_api_key,
    create_test_entity, create_test_entity_definition, get_test_user_username,
};
pub use queue::{make_workflow_service, test_queue_client, test_queue_client_async};
