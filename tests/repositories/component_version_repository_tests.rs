#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_persistence::ComponentVersionRepository;
use r_data_core_test_support::setup_test_db;

#[tokio::test]
async fn test_upsert_creates_new_version() {
    let pool = setup_test_db().await;
    let repo = ComponentVersionRepository::new(pool.pool.clone());

    // Insert a new component version
    repo.upsert("test-worker", "1.0.0").await.unwrap();

    // Verify it was created
    let version = repo.get("test-worker").await.unwrap();
    assert!(version.is_some());
    let version = version.unwrap();
    assert_eq!(version.component_name, "test-worker");
    assert_eq!(version.version, "1.0.0");
}

#[tokio::test]
async fn test_upsert_updates_existing_version() {
    let pool = setup_test_db().await;
    let repo = ComponentVersionRepository::new(pool.pool.clone());

    // Insert initial version
    repo.upsert("test-maintenance", "1.0.0").await.unwrap();
    let v1 = repo.get("test-maintenance").await.unwrap().unwrap();

    // Small delay to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Update to new version
    repo.upsert("test-maintenance", "1.1.0").await.unwrap();
    let v2 = repo.get("test-maintenance").await.unwrap().unwrap();

    // Version should be updated
    assert_eq!(v2.version, "1.1.0");
    // Timestamp should be later
    assert!(v2.last_seen_at >= v1.last_seen_at);
}

#[tokio::test]
async fn test_get_returns_none_for_unknown_component() {
    let pool = setup_test_db().await;
    let repo = ComponentVersionRepository::new(pool.pool.clone());

    let version = repo.get("nonexistent-component").await.unwrap();
    assert!(version.is_none());
}

#[tokio::test]
async fn test_get_all_returns_all_components() {
    let pool = setup_test_db().await;
    let repo = ComponentVersionRepository::new(pool.pool.clone());

    // Clear any existing entries first
    sqlx::query("DELETE FROM component_versions")
        .execute(&pool.pool)
        .await
        .unwrap();

    // Insert multiple components
    repo.upsert("alpha-component", "1.0.0").await.unwrap();
    repo.upsert("beta-component", "2.0.0").await.unwrap();
    repo.upsert("gamma-component", "3.0.0").await.unwrap();

    // Get all should return all three, ordered by name
    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].component_name, "alpha-component");
    assert_eq!(all[1].component_name, "beta-component");
    assert_eq!(all[2].component_name, "gamma-component");
}

#[tokio::test]
async fn test_get_all_returns_empty_when_no_components() {
    let pool = setup_test_db().await;
    let repo = ComponentVersionRepository::new(pool.pool.clone());

    // Clear any existing entries
    sqlx::query("DELETE FROM component_versions")
        .execute(&pool.pool)
        .await
        .unwrap();

    let all = repo.get_all().await.unwrap();
    assert!(all.is_empty());
}
