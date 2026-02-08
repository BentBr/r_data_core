#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
// Will contain API integration tests

// API tests
pub mod admin_auth_tests;
pub mod api_key_integration_tests;
pub mod api_key_routes_tests;
pub mod authentication_tests;
pub mod dynamic_entity_api_tests;
pub mod dynamic_entity_routes_tests;
pub mod entity_definition_integration_tests;
pub mod entity_definitions;
pub mod error_handling_tests;
pub mod meta;
pub mod provider_workflow_endpoints_tests;
pub mod query_validation_integration_tests;
pub mod refresh_token_integration_tests;
pub mod roles;
pub mod users;
pub mod workflows;
pub mod workflows_routes_tests;
