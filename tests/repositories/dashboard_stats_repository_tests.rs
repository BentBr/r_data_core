#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_core::entity_definition::definition::{EntityDefinition, EntityDefinitionParams};
use r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core_persistence::{
    DashboardStatsRepository, DashboardStatsRepositoryTrait, EntityDefinitionRepository,
};
use r_data_core_test_support::{create_test_admin_user, setup_test_db};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

async fn setup_test_repository() -> (Arc<DashboardStatsRepository>, PgPool) {
    let pool = setup_test_db().await;
    let repository = Arc::new(DashboardStatsRepository::new(pool.clone()));
    (repository, pool)
}

#[tokio::test]
async fn test_get_dashboard_stats_empty() {
    let (repo, _pool) = setup_test_repository().await;

    let stats = repo.get_dashboard_stats().await.unwrap();

    assert_eq!(stats.entity_definitions_count, 0);
    assert_eq!(stats.entities.total, 0);
    assert_eq!(stats.entities.by_type.len(), 0);
    assert_eq!(stats.workflows.total, 0);
    assert_eq!(stats.workflows.workflows.len(), 0);
    assert_eq!(stats.online_users_count, 0);
}

#[tokio::test]
async fn test_get_dashboard_stats_with_data() {
    let (repo, pool) = setup_test_repository().await;

    // Create admin user for created_by references
    let admin_user_uuid = create_test_admin_user(&pool).await.unwrap();

    // Create entity definition
    let entity_def_repo = EntityDefinitionRepository::new(pool.clone());
    let entity_def = EntityDefinition::from_params(EntityDefinitionParams {
        entity_type: "test_entity".to_string(),
        display_name: "Test Entity".to_string(),
        description: None,
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![],
        created_by: admin_user_uuid,
    });
    EntityDefinitionRepositoryTrait::create(&entity_def_repo, &entity_def)
        .await
        .unwrap();

    // Create entity in registry
    let entity_uuid = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO entities_registry (uuid, entity_type, path, entity_key, created_by, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
        "#,
        entity_uuid,
        "test_entity",
        "/",
        format!("test-entity-{}", entity_uuid.simple()),
        admin_user_uuid
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create workflow
    sqlx::query(
        r"
        INSERT INTO workflows (uuid, name, kind, enabled, created_by, created_at, updated_at)
        VALUES ($1, $2, $3::workflow_kind, $4, $5, NOW(), NOW())
        ",
    )
    .bind(Uuid::now_v7())
    .bind("Test Workflow")
    .bind("consumer")
    .bind(true)
    .bind(admin_user_uuid)
    .execute(&pool)
    .await
    .unwrap();

    // Create refresh token (online user)
    sqlx::query!(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at, is_revoked, created_at)
        VALUES ($1, $2, NOW() + INTERVAL '1 day', false, NOW())
        "#,
        admin_user_uuid,
        "test_hash"
    )
    .execute(&pool)
    .await
    .unwrap();

    let stats = repo.get_dashboard_stats().await.unwrap();

    assert_eq!(stats.entity_definitions_count, 1);
    assert_eq!(stats.entities.total, 1);
    assert_eq!(stats.entities.by_type.len(), 1);
    assert_eq!(stats.entities.by_type[0].entity_type, "test_entity");
    assert_eq!(stats.entities.by_type[0].count, 1);
    assert_eq!(stats.workflows.total, 1);
    assert_eq!(stats.workflows.workflows.len(), 1);
    assert_eq!(stats.workflows.workflows[0].name, "Test Workflow");
    assert_eq!(stats.online_users_count, 1);
}
