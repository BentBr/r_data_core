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
// Bug documentation — create/update bind timestamps as text
// ---------------------------------------------------------------------------

/// `EntityRepository::create` extracts `created_at`/`updated_at` from the
/// serialized JSON as `&str` and binds them as text parameters, but the
/// backing column is `TIMESTAMPTZ`.  Postgres refuses implicit text→timestamptz
/// casts for parameterized queries (error code 42804).
///
/// BUG: owning layer is `persistence` (`crates/persistence/src/repository.rs`).
/// The fix is to parse the timestamp strings into `time::OffsetDateTime` (or
/// bind them with an explicit `::timestamptz` cast) before binding.
#[tokio::test]
#[serial]
async fn test_create_fails_due_to_timestamp_text_binding_bug() -> Result<()> {
    use serde_json::json;
    use time::OffsetDateTime;

    use super::create_test_table;

    let db = setup_test_db().await;
    clear_test_db(&db).await?;
    create_test_table(&db.pool).await;

    let r: EntityRepository<TestItem> = db.pool.repository_with_table("test_items");

    // Construct a JSON-serializable value that matches what create() reads.
    // The repo extracts "uuid", "path", "created_at", "updated_at", etc.
    let now = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("format");

    let fake_entity = json!({
        "uuid": Uuid::now_v7().to_string(),
        "path": "/test",
        "created_at": now,
        "updated_at": now,
        "created_by": Uuid::now_v7().to_string(),
        "updated_by": null,
        "published": false,
        "version": 1_i64,
        "custom_fields": {}
    });

    // Deserialize to TestItem so we can call create().
    let item: TestItem = serde_json::from_value(fake_entity).expect("deserialize");

    let result = r.create(&item).await;

    // BUG: this errors with Postgres 42804 "expression is of type text"
    // because create() binds created_at/updated_at as text parameters.
    assert!(
        result.is_err(),
        "EntityRepository::create should fail (bug: timestamps bound as text)"
    );

    Ok(())
}
