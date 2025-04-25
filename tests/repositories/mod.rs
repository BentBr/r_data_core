pub mod admin_user_repository_tests;
pub mod api_key_repository_tests;
pub mod class_definition_repository_tests;

use crate::common::setup_test_db;
use r_data_core::api::admin::class_definitions::repository::ClassDefinitionRepository;
use r_data_core::entity::class::repository_trait::ClassDefinitionRepositoryTrait;
use std::sync::Arc;

/// Get a ClassDefinitionRepository for testing
#[allow(dead_code)]
pub async fn get_class_definition_repository() -> Arc<dyn ClassDefinitionRepositoryTrait> {
    let pool = setup_test_db().await;
    Arc::new(ClassDefinitionRepository::new(pool))
}
