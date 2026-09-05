#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
// Will contain API integration tests

// API tests
pub mod admin_auth_tests;
pub mod admin_login_tests;
pub mod api_key_integration_tests;
pub mod api_key_routes_tests;
pub mod authentication_tests;
pub mod dsl_options_tests;
pub mod dynamic_entity_api_tests;
pub mod dynamic_entity_routes_tests;
pub mod email_templates_tests;
pub mod entity_definition_crud_tests;
pub mod entity_definition_integration_tests;
pub mod entity_definitions;
pub mod error_handling_tests;
pub mod meta;
pub mod provider_workflow_endpoints_tests;
pub mod public_entities_tests;
pub mod query_validation_integration_tests;
pub mod refresh_token_integration_tests;
pub mod roles;
pub mod roles_crud_tests;
pub mod system_info_tests;
pub mod system_settings_more_tests;
pub mod system_settings_tests;
pub mod users;
pub mod versioning_routes_tests;
pub mod workflow_listing_tests;
pub mod workflows;
pub mod workflows_routes_tests;
