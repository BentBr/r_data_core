#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::collections::HashMap;

use r_data_core_core::error::Result;
use r_data_core_core::public_api::AdvancedEntityQuery;
use r_data_core_persistence::{DynamicEntityQueryRepository, DynamicEntityRepository};
use r_data_core_test_support::{clear_test_db, setup_test_db};
use serial_test::serial;

use super::{make_entity, setup_entity_type};

// ── no filters — all entities returned ───────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_query_returns_all_entities_with_no_filters() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_all").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());

    for i in 0..3_i64 {
        entity_repo
            .create(&make_entity(&def, &format!("entity-{i}"), i * 10))
            .await?;
    }

    let q = AdvancedEntityQuery {
        filter: None,
        limit: None,
        offset: None,
        sort_by: None,
        sort_direction: None,
    };
    let results = query_repo.query_entities(&def.entity_type, &q).await?;
    assert_eq!(results.len(), 3);

    Ok(())
}

// ── filter by string field ────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_query_filters_by_string_field() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_filter_str").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());

    entity_repo.create(&make_entity(&def, "Alice", 10)).await?;
    entity_repo.create(&make_entity(&def, "Bob", 20)).await?;

    let mut filter = HashMap::new();
    filter.insert("name".to_string(), serde_json::json!("Alice"));

    let q = AdvancedEntityQuery {
        filter: Some(filter),
        limit: None,
        offset: None,
        sort_by: None,
        sort_direction: None,
    };
    let results = query_repo.query_entities(&def.entity_type, &q).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].field_data.get("name").and_then(|v| v.as_str()),
        Some("Alice")
    );

    Ok(())
}

// ── pagination: limit & offset ────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_query_pagination_limit_and_offset() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_page").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());

    for i in 0..5_i64 {
        entity_repo
            .create(&make_entity(&def, &format!("p-{i}"), i))
            .await?;
    }

    let q_limit = AdvancedEntityQuery {
        filter: None,
        limit: Some(2),
        offset: Some(0),
        sort_by: None,
        sort_direction: None,
    };
    let page1 = query_repo
        .query_entities(&def.entity_type, &q_limit)
        .await?;
    assert_eq!(page1.len(), 2);

    let q_offset = AdvancedEntityQuery {
        filter: None,
        limit: Some(2),
        offset: Some(4),
        sort_by: None,
        sort_direction: None,
    };
    let page_last = query_repo
        .query_entities(&def.entity_type, &q_offset)
        .await?;
    assert_eq!(page_last.len(), 1);

    Ok(())
}

// ── limit capped at 1000 ──────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_query_limit_is_capped_at_1000() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_cap").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());

    for i in 0..3_i64 {
        entity_repo
            .create(&make_entity(&def, &format!("c-{i}"), i))
            .await?;
    }

    // A limit larger than 1000 is capped internally; must still return all rows.
    let q = AdvancedEntityQuery {
        filter: None,
        limit: Some(99_999),
        offset: None,
        sort_by: None,
        sort_direction: None,
    };
    let results = query_repo.query_entities(&def.entity_type, &q).await?;
    assert_eq!(results.len(), 3);

    Ok(())
}
