#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_persistence::ApiKeyRepository;
use r_data_core_persistence::ApiKeyRepositoryTrait;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_list_by_user_returns_correct_keys() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(std::sync::Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    let (key1_uuid, _) = repo
        .create_new_api_key(&random_string("list_k1"), "desc1", user_uuid, 30)
        .await?;
    let (key2_uuid, _) = repo
        .create_new_api_key(&random_string("list_k2"), "desc2", user_uuid, 30)
        .await?;

    let keys = repo.list_by_user(user_uuid, 10, 0, None, None).await?;
    let key_uuids: Vec<Uuid> = keys.iter().map(|k| k.uuid).collect();
    assert!(key_uuids.contains(&key1_uuid));
    assert!(key_uuids.contains(&key2_uuid));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_by_user_pagination() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(std::sync::Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    for i in 0..5 {
        repo.create_new_api_key(&random_string(&format!("page_k{i}")), "d", user_uuid, 30)
            .await?;
    }

    let page1 = repo.list_by_user(user_uuid, 2, 0, None, None).await?;
    let page2 = repo.list_by_user(user_uuid, 2, 2, None, None).await?;
    assert_eq!(page1.len(), 2);
    assert_eq!(page2.len(), 2);

    let uuids1: Vec<Uuid> = page1.iter().map(|k| k.uuid).collect();
    for k in &page2 {
        assert!(!uuids1.contains(&k.uuid), "Pages should not overlap");
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_by_user_unlimited() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(std::sync::Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    for i in 0..3 {
        repo.create_new_api_key(&random_string(&format!("unlim_k{i}")), "d", user_uuid, 30)
            .await?;
    }

    // limit = -1 means unlimited
    let keys = repo.list_by_user(user_uuid, -1, 0, None, None).await?;
    assert_eq!(keys.len(), 3);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_by_user_empty_for_unknown_user() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(std::sync::Arc::new(pool.pool.clone()));
    let unknown = Uuid::now_v7();

    let keys = repo.list_by_user(unknown, 100, 0, None, None).await?;
    assert!(keys.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_count_by_user() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(std::sync::Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    assert_eq!(repo.count_by_user(user_uuid).await?, 0);

    repo.create_new_api_key(&random_string("cnt_k1"), "d", user_uuid, 30)
        .await?;
    assert_eq!(repo.count_by_user(user_uuid).await?, 1);

    repo.create_new_api_key(&random_string("cnt_k2"), "d", user_uuid, 30)
        .await?;
    assert_eq!(repo.count_by_user(user_uuid).await?, 2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_by_user_sort_by_name() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = ApiKeyRepository::new(std::sync::Arc::new(pool.pool.clone()));
    let user_uuid = create_test_admin_user(&pool).await?;

    repo.create_new_api_key("zzz_sort_key", "d", user_uuid, 30)
        .await?;
    repo.create_new_api_key("aaa_sort_key", "d", user_uuid, 30)
        .await?;

    let keys_asc = repo
        .list_by_user(
            user_uuid,
            10,
            0,
            Some("name".to_string()),
            Some("ASC".to_string()),
        )
        .await?;

    assert_eq!(keys_asc.len(), 2);
    assert_eq!(keys_asc[0].name, "aaa_sort_key");
    assert_eq!(keys_asc[1].name, "zzz_sort_key");

    Ok(())
}
