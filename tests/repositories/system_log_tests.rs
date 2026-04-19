#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_core::system_log::{SystemLogResourceType, SystemLogStatus, SystemLogType};
use r_data_core_persistence::{SystemLogRepository, SystemLogRepositoryTrait};
use r_data_core_test_support::{clear_test_db, setup_test_db};
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_system_log_insert_and_query() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = SystemLogRepository::new(pool.pool.clone());

    // Insert a log entry
    let resource_uuid = Uuid::now_v7();
    let log_uuid = repo
        .insert(
            None,
            SystemLogStatus::Success,
            SystemLogType::AuthEvent,
            SystemLogResourceType::AdminUser,
            Some(resource_uuid),
            "User logged in",
            Some(serde_json::json!({"action": "login"})),
        )
        .await?;

    // Get by UUID
    let log = repo
        .get_by_uuid(log_uuid)
        .await?
        .expect("log entry should exist after insert");
    assert_eq!(log.summary, "User logged in");
    assert_eq!(log.status, SystemLogStatus::Success);
    assert_eq!(log.log_type, SystemLogType::AuthEvent);
    assert_eq!(log.resource_type, SystemLogResourceType::AdminUser);
    assert_eq!(log.resource_uuid, Some(resource_uuid));
    assert!(log.created_by.is_none());
    assert!(log.details.is_some());

    // List paginated — no filters
    let (logs, total) = repo.list_paginated(10, 0, None, None, None).await?;
    assert!(total >= 1, "total should reflect the inserted log");
    assert!(!logs.is_empty());

    // List with log_type filter
    let (filtered, _) = repo
        .list_paginated(10, 0, Some(SystemLogType::AuthEvent), None, None)
        .await?;
    assert!(
        !filtered.is_empty(),
        "filtered result should include the auth_event log"
    );
    assert!(filtered
        .iter()
        .all(|l| l.log_type == SystemLogType::AuthEvent));

    // List with status filter
    let (by_status, _) = repo
        .list_paginated(10, 0, None, None, Some(SystemLogStatus::Success))
        .await?;
    assert!(!by_status.is_empty());
    assert!(by_status
        .iter()
        .all(|l| l.status == SystemLogStatus::Success));

    // List with resource_type filter
    let (by_resource, _) = repo
        .list_paginated(10, 0, None, Some(SystemLogResourceType::AdminUser), None)
        .await?;
    assert!(!by_resource.is_empty());

    // delete_older_than_days with 1 day should NOT delete a log inserted seconds ago
    let deleted = repo.delete_older_than_days(1).await?;
    assert_eq!(
        deleted, 0,
        "recently inserted log should not be deleted by 1-day retention"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_system_log_with_created_by() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = SystemLogRepository::new(pool.pool.clone());

    let actor_uuid = Uuid::now_v7();
    let log_uuid = repo
        .insert(
            Some(actor_uuid),
            SystemLogStatus::Failed,
            SystemLogType::EmailSent,
            SystemLogResourceType::Email,
            None,
            "Failed to send email",
            None,
        )
        .await?;

    let log = repo
        .get_by_uuid(log_uuid)
        .await?
        .expect("log entry should exist");
    assert_eq!(log.created_by, Some(actor_uuid));
    assert_eq!(log.status, SystemLogStatus::Failed);
    assert_eq!(log.log_type, SystemLogType::EmailSent);
    assert!(log.details.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_system_log_pagination() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = SystemLogRepository::new(pool.pool.clone());

    // Insert 5 log entries
    for i in 0..5u8 {
        repo.insert(
            None,
            SystemLogStatus::Success,
            SystemLogType::EntityCreated,
            SystemLogResourceType::EntityDefinition,
            None,
            &format!("Entity created {i}"),
            None,
        )
        .await?;
    }

    // Page 1: limit 3, offset 0
    let (page1, total) = repo.list_paginated(3, 0, None, None, None).await?;
    assert_eq!(total, 5);
    assert_eq!(page1.len(), 3);

    // Page 2: limit 3, offset 3
    let (page2, _) = repo.list_paginated(3, 3, None, None, None).await?;
    assert_eq!(page2.len(), 2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_system_log_get_by_uuid_not_found() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = SystemLogRepository::new(pool.pool.clone());

    let result = repo.get_by_uuid(Uuid::now_v7()).await?;
    assert!(
        result.is_none(),
        "get_by_uuid should return None for unknown UUID"
    );

    Ok(())
}
