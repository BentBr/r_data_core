#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
pub mod adapters;
pub mod api;
pub mod cache;
pub mod license;
pub mod repositories;
pub mod services;
pub mod statistics;
pub mod utils;
pub mod worker;

// Top level integration test modules
mod clear_cache_tests;
mod dsl_integration_tests;
mod dynamic_entity_api_tests;
mod e2e_workflow_queue_tests;
mod entity_type_column_test;
mod hash_passwords;
mod queue_integration_tests;
mod redis_cache_tests;
mod validation_tests;
mod versioning_tests;
mod worker_repository_tests;
mod workflow_entity_audit_tests;
mod workflows_audit_fields_tests;
