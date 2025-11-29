pub mod admin_user_repository_tests;
pub mod api_key_repository_tests;
pub mod dynamic_entity_repository_tests;
pub mod entity_definition_repository_tests;
pub mod filter_entities_tests;
pub mod refresh_token_repository_tests;
pub mod version_repository_tests;

use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_test_support::setup_test_db;
use sqlx::PgPool;
use std::sync::Arc;

/// Test repository structure that provides both repository and pool access
pub struct TestRepository {
    pub repository: Arc<EntityDefinitionRepository>,
    pub db_pool: PgPool,
}

pub async fn get_entity_definition_repository_with_pool() -> TestRepository {
    let pool = setup_test_db().await;
    let repository = Arc::new(EntityDefinitionRepository::new(pool.clone()));
    TestRepository {
        repository,
        db_pool: pool,
    }
}
