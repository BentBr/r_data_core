pub mod admin_user_repository_tests;
pub mod api_key_repository_tests;
pub mod class_definition_repository_tests;

use crate::common::setup_test_db;
use r_data_core::api::admin::class_definitions::repository::ClassDefinitionRepository;
use r_data_core::entity::class::repository_trait::ClassDefinitionRepositoryTrait;
use sqlx::PgPool;
use std::sync::Arc;

/// Get a ClassDefinitionRepository for testing
#[allow(dead_code)]
pub async fn get_class_definition_repository() -> Arc<dyn ClassDefinitionRepositoryTrait> {
    let pool = setup_test_db().await;
    Arc::new(ClassDefinitionRepository::new(pool))
}

// Add a function that returns a repository with direct access to the pool
pub struct TestRepository {
    pub repository: Arc<ClassDefinitionRepository>,
    pub db_pool: PgPool,
}

pub async fn get_class_definition_repository_with_pool() -> TestRepository {
    let pool = setup_test_db().await;
    let repository = Arc::new(ClassDefinitionRepository::new(pool.clone()));
    TestRepository {
        repository,
        db_pool: pool,
    }
}
