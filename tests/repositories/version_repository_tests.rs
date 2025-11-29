use r_data_core_persistence::VersionRepository;
use sqlx::Row;
use uuid::Uuid;

use r_data_core_test_support::setup_test_db;

#[tokio::test]
async fn test_version_repository_list_and_get() {
    let pool = setup_test_db().await;
    let repo = VersionRepository::new(pool.clone());
    let entity_uuid = Uuid::now_v7();

    // First create the entity in entities_registry (required for foreign key)
    let created_by = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO entities_registry (uuid, entity_type, entity_key, path, created_at, created_by) VALUES ($1, $2, $3, $4, NOW(), $5) ON CONFLICT DO NOTHING",
    )
    .bind(entity_uuid)
    .bind("dynamic_entity")
    .bind(format!("test-entity-{}", entity_uuid.simple()))
    .bind("/")
    .bind(created_by)
    .execute(&pool)
    .await
    .unwrap();

    // Seed two versions
    for v in 1..=2 {
        let _ = sqlx::query(
            "INSERT INTO entities_versions (entity_uuid, entity_type, version_number, data, created_at) VALUES ($1, $2, $3, $4, NOW()) ON CONFLICT DO NOTHING",
        )
        .bind(entity_uuid)
        .bind("dynamic_entity")
        .bind(v)
        .bind(serde_json::json!({"v": v}))
        .execute(&pool)
        .await
        .unwrap();
    }

    // List should return 2 rows with 2 then 1
    let list = repo.list_entity_versions(entity_uuid).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].version_number, 2);
    assert_eq!(list[1].version_number, 1);

    // Get specific version
    let v1 = repo
        .get_entity_version(entity_uuid, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(v1.version_number, 1);
    assert_eq!(v1.data.get("v").and_then(|x| x.as_i64()).unwrap(), 1);
}
