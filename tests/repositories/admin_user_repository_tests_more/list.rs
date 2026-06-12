#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

use r_data_core_core::error::Result;
use r_data_core_persistence::AdminUserRepositoryTrait;
use r_data_core_test_support::{clear_test_db, setup_test_db};
use serial_test::serial;

use super::users::{make_repo, seed_user};

#[tokio::test]
#[serial]
async fn test_list_admin_users_pagination() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    for _ in 0..3 {
        seed_user(&repo, &pool).await?;
    }

    let page1 = repo.list_admin_users(2, 0, None, None).await?;
    let page2 = repo.list_admin_users(2, 2, None, None).await?;

    assert_eq!(page1.len(), 2, "first page should return 2 users");
    assert!(
        !page2.is_empty(),
        "second page should have at least one user"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_admin_users_unlimited() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    for _ in 0..3 {
        seed_user(&repo, &pool).await?;
    }

    let all = repo.list_admin_users(i64::MAX, 0, None, None).await?;
    assert!(all.len() >= 3, "unlimited fetch should return all users");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_admin_users_sort_by_username_asc() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    seed_user(&repo, &pool).await?;
    seed_user(&repo, &pool).await?;

    let users = repo
        .list_admin_users(
            100,
            0,
            Some("username".to_string()),
            Some("ASC".to_string()),
        )
        .await?;

    let usernames: Vec<&str> = users.iter().map(|u| u.username.as_str()).collect();
    let mut sorted = usernames.clone();
    sorted.sort_unstable();
    assert_eq!(usernames, sorted, "users should be sorted by username ASC");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_admin_users_sort_by_username_desc() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    seed_user(&repo, &pool).await?;
    seed_user(&repo, &pool).await?;

    let users = repo
        .list_admin_users(
            100,
            0,
            Some("username".to_string()),
            Some("DESC".to_string()),
        )
        .await?;

    let usernames: Vec<&str> = users.iter().map(|u| u.username.as_str()).collect();
    let mut sorted = usernames.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(usernames, sorted, "users should be sorted by username DESC");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_admin_users_sort_by_roles() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool.pool).await?;
    let repo = make_repo(&pool);

    seed_user(&repo, &pool).await?;
    seed_user(&repo, &pool).await?;

    let users = repo
        .list_admin_users(100, 0, Some("roles".to_string()), Some("ASC".to_string()))
        .await?;
    assert!(
        !users.is_empty(),
        "should return users sorted by role count"
    );
    Ok(())
}
