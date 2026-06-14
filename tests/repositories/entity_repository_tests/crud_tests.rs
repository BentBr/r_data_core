use super::{create_test_table, repo, seed_item, setup_test_db, Result, Uuid};
use r_data_core_test_support::clear_test_db;
use serial_test::serial;

// ---------------------------------------------------------------------------
// Tests — get_by_uuid
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_get_by_uuid_found() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    let uuid = seed_item(&db.pool, "/hello", false).await;
    let r = repo(&db.pool);

    let fetched = r.get_by_uuid(&uuid).await?;
    assert_eq!(fetched.uuid, uuid);
    assert_eq!(fetched.path, "/hello");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_by_uuid_not_found_returns_error() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    let r = repo(&db.pool);
    let result = r.get_by_uuid(&Uuid::now_v7()).await;

    assert!(result.is_err(), "expected Err for missing uuid");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests — list
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_list_no_filter_returns_all() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    seed_item(&db.pool, "/a", false).await;
    seed_item(&db.pool, "/b", true).await;
    seed_item(&db.pool, "/c", false).await;

    let r = repo(&db.pool);
    let items = r.list(None, None, None, None).await?;
    assert_eq!(items.len(), 3);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_empty_table() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    let r = repo(&db.pool);
    let items = r.list(None, None, None, None).await?;
    assert!(items.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_with_limit_and_offset() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    for i in 0..5_u8 {
        seed_item(&db.pool, &format!("/{i}"), false).await;
    }

    let r = repo(&db.pool);
    let page = r.list(None, None, Some(2), Some(1)).await?;
    assert_eq!(page.len(), 2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_with_filter() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    seed_item(&db.pool, "/pub", true).await;
    seed_item(&db.pool, "/unpub", false).await;

    let r = repo(&db.pool);
    let results = r.list(Some("published = true"), None, None, None).await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].published);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_with_sort() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    seed_item(&db.pool, "/z", false).await;
    seed_item(&db.pool, "/a", false).await;

    let r = repo(&db.pool);
    let sorted = r.list(None, Some("path ASC"), None, None).await?;
    assert_eq!(sorted.len(), 2);
    assert_eq!(sorted[0].path, "/a");
    assert_eq!(sorted[1].path, "/z");

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests — count
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_count_no_filter() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    seed_item(&db.pool, "/1", false).await;
    seed_item(&db.pool, "/2", false).await;

    let r = repo(&db.pool);
    let count = r.count(None).await?;
    assert_eq!(count, 2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_count_with_filter() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    seed_item(&db.pool, "/p", true).await;
    seed_item(&db.pool, "/q", false).await;

    let r = repo(&db.pool);
    let count = r.count(Some("published = true")).await?;
    assert_eq!(count, 1);

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests — delete
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_delete_removes_entity() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    let uuid = seed_item(&db.pool, "/del", false).await;
    let r = repo(&db.pool);

    r.delete(&uuid).await?;

    let result = r.get_by_uuid(&uuid).await;
    assert!(result.is_err(), "expected not-found after delete");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_delete_nonexistent_is_ok() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    let r = repo(&db.pool);
    // DELETE on unknown uuid — must not error
    r.delete(&Uuid::now_v7()).await?;

    Ok(())
}
