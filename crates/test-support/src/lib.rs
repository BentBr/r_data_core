#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod api;
pub mod database;
pub mod entities;
pub mod queue;

// Re-export commonly used items
// pub use api::wrap_api_state; // Commented out due to circular dependency - use directly in tests
pub use database::{
    clear_entity_definitions, clear_refresh_tokens, clear_test_db, fast_clear_test_db,
    setup_test_db, unique_entity_type, GLOBAL_TEST_MUTEX,
};
pub use entities::{
    create_test_admin_user, create_test_api_key, create_test_entity,
    create_test_entity_definition,
};
pub use queue::test_queue_client_async;
