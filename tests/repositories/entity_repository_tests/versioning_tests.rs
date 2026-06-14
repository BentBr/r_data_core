use super::{setup_test_db, EntityRepository, PgPoolExtension, Result, TestItem, Uuid};
use r_data_core_test_support::clear_test_db;
use serial_test::serial;

// ---------------------------------------------------------------------------
// Tests — versioning
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_get_version_and_list_versions() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;

    let r: EntityRepository<TestItem> = db.pool.repository_with_table("test_items");

    // Seed entities_registry so the FK on entities_versions is satisfied.
    let entity_uuid = Uuid::now_v7();
    let creator = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO entities_registry
            (uuid, entity_type, path, entity_key, created_by, version)
         VALUES ($1, 'test_type', '/', $2, $3, 1)",
    )
    .bind(entity_uuid)
    .bind(entity_uuid.to_string())
    .bind(creator)
    .execute(&db.pool)
    .await
    .expect("seed entities_registry");

    sqlx::query(
        "INSERT INTO entities_versions
            (entity_uuid, entity_type, version_number, data, created_by)
         VALUES ($1, 'test_type', 1, '{\"x\":1}'::jsonb, $2)",
    )
    .bind(entity_uuid)
    .bind(creator)
    .execute(&db.pool)
    .await
    .expect("seed entities_versions");

    let ver = r.get_version(&entity_uuid, 1).await?;
    assert_eq!(ver.entity_uuid, entity_uuid);
    assert_eq!(ver.version_number, 1);

    let versions = r.list_versions(&entity_uuid).await?;
    assert_eq!(versions.len(), 1);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_get_version_not_found_returns_error() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;

    let r: EntityRepository<TestItem> = db.pool.repository_with_table("test_items");
    let result = r.get_version(&Uuid::now_v7(), 99).await;
    assert!(result.is_err(), "expected error for missing version");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_versions_empty_for_unknown_entity() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;

    let r: EntityRepository<TestItem> = db.pool.repository_with_table("test_items");
    let versions = r.list_versions(&Uuid::now_v7()).await?;
    assert!(versions.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_list_versions_multiple_descending() -> Result<()> {
    let db = setup_test_db().await;
    clear_test_db(&db).await?;

    let r: EntityRepository<TestItem> = db.pool.repository_with_table("test_items");

    let entity_uuid = Uuid::now_v7();
    let creator = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO entities_registry
            (uuid, entity_type, path, entity_key, created_by, version)
         VALUES ($1, 'multi_v', '/', $2, $3, 3)",
    )
    .bind(entity_uuid)
    .bind(entity_uuid.to_string())
    .bind(creator)
    .execute(&db.pool)
    .await
    .expect("seed registry");

    for v in 1..=3_i32 {
        sqlx::query(
            "INSERT INTO entities_versions
                (entity_uuid, entity_type, version_number, data, created_by)
             VALUES ($1, 'multi_v', $2, '{}'::jsonb, $3)",
        )
        .bind(entity_uuid)
        .bind(v)
        .bind(creator)
        .execute(&db.pool)
        .await
        .expect("seed version");
    }

    let versions = r.list_versions(&entity_uuid).await?;
    assert_eq!(versions.len(), 3);
    // Must be in DESC order (list_versions orders by version_number DESC)
    assert!(versions[0].version_number > versions[1].version_number);
    assert!(versions[1].version_number > versions[2].version_number);

    Ok(())
}

// ---------------------------------------------------------------------------
// create() persists timestamps — regression guard for the 42804 binding bug
// ---------------------------------------------------------------------------

/// Regression test: `EntityRepository::create` must persist an entity whose
/// `created_at`/`updated_at` are real `time::OffsetDateTime` values.
///
/// Previously `create` extracted those fields as `&str` and bound them as text
/// into `TIMESTAMPTZ` columns, which (a) never matched `time`'s array-based
/// serde encoding and (b) failed with Postgres error 42804 when it was a string.
/// The repository now deserializes them back into `OffsetDateTime` and binds the
/// typed value. This test creates a row and reads it back to prove the round-trip.
#[tokio::test]
#[serial]
async fn test_create_persists_entity_and_timestamps() -> Result<()> {
    use serde_json::json;
    use time::OffsetDateTime;

    use super::create_test_table;

    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    let r: EntityRepository<TestItem> = db.pool.repository_with_table("test_items");

    let uuid = Uuid::now_v7();
    let created_by = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();
    let item = TestItem {
        uuid,
        path: "/created/via/repo".to_string(),
        created_at: now,
        updated_at: now,
        created_by: Some(created_by),
        updated_by: None,
        published: true,
        version: 1,
        custom_fields: json!({ "k": "v" }),
    };

    // create() must succeed (no 42804) and return the row's uuid.
    let returned = r.create(&item).await?;
    assert_eq!(returned, uuid);

    // Read it back and confirm every field round-tripped.
    let fetched = r.get_by_uuid(&uuid).await?;
    assert_eq!(fetched.path, "/created/via/repo");
    assert!(fetched.published);
    assert_eq!(fetched.version, 1);
    assert_eq!(fetched.created_by, Some(created_by));
    assert_eq!(fetched.custom_fields, json!({ "k": "v" }));
    // Timestamps persist at Postgres microsecond precision; compare whole seconds.
    assert_eq!(fetched.created_at.unix_timestamp(), now.unix_timestamp());
    assert_eq!(fetched.updated_at.unix_timestamp(), now.unix_timestamp());

    Ok(())
}

/// Regression test for the same 42804 binding bug on the `update` path:
/// `update` must persist the new `updated_at` `OffsetDateTime` and mutated fields.
#[tokio::test]
#[serial]
async fn test_update_persists_changes_and_updated_at() -> Result<()> {
    use serde_json::json;
    use time::OffsetDateTime;

    use super::create_test_table;

    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    let r: EntityRepository<TestItem> = db.pool.repository_with_table("test_items");

    let uuid = Uuid::now_v7();
    let created = OffsetDateTime::now_utc();
    let original = TestItem {
        uuid,
        path: "/before".to_string(),
        created_at: created,
        updated_at: created,
        created_by: None,
        updated_by: None,
        published: false,
        version: 1,
        custom_fields: json!({}),
    };
    r.create(&original).await?;

    let later = OffsetDateTime::now_utc();
    let updated = TestItem {
        path: "/after".to_string(),
        updated_at: later,
        published: true,
        version: 2,
        custom_fields: json!({ "changed": true }),
        ..original
    };
    r.update(&uuid, &updated).await?;

    let fetched = r.get_by_uuid(&uuid).await?;
    assert_eq!(fetched.path, "/after");
    assert!(fetched.published);
    assert_eq!(fetched.version, 2);
    assert_eq!(fetched.custom_fields, json!({ "changed": true }));
    assert_eq!(fetched.updated_at.unix_timestamp(), later.unix_timestamp());

    Ok(())
}
