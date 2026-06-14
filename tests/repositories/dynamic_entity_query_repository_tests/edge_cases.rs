#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::collections::HashMap;

use r_data_core_core::error::Result;
use r_data_core_core::public_api::AdvancedEntityQuery;
use r_data_core_persistence::{DynamicEntityQueryRepository, DynamicEntityRepository};
use r_data_core_test_support::{clear_test_db, setup_test_db};
use serial_test::serial;

use super::{make_entity, setup_entity_type};

// ── sort ASC / DESC ───────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_query_sort_asc_and_desc() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_sort").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());

    for name in &["Charlie", "Alice", "Bob"] {
        entity_repo.create(&make_entity(&def, name, 0)).await?;
    }

    let q_asc = AdvancedEntityQuery {
        filter: None,
        limit: Some(10),
        offset: None,
        sort_by: Some("name".to_string()),
        sort_direction: Some("ASC".to_string()),
    };
    let asc_results = query_repo.query_entities(&def.entity_type, &q_asc).await?;
    let asc_names: Vec<&str> = asc_results
        .iter()
        .filter_map(|e| e.field_data.get("name").and_then(|v| v.as_str()))
        .collect();
    let mut sorted = asc_names.clone();
    sorted.sort_unstable();
    assert_eq!(asc_names, sorted);

    let q_desc = AdvancedEntityQuery {
        filter: None,
        limit: Some(10),
        offset: None,
        sort_by: Some("name".to_string()),
        sort_direction: Some("DESC".to_string()),
    };
    let desc_results = query_repo.query_entities(&def.entity_type, &q_desc).await?;
    let desc_names: Vec<&str> = desc_results
        .iter()
        .filter_map(|e| e.field_data.get("name").and_then(|v| v.as_str()))
        .collect();
    let mut sorted_rev = desc_names.clone();
    sorted_rev.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(desc_names, sorted_rev);

    Ok(())
}

// ── invalid sort_by falls back gracefully ─────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_query_invalid_sort_by_falls_back_to_created_at() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_badsort").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());

    entity_repo.create(&make_entity(&def, "X", 1)).await?;

    // sort_by containing special characters must fall back to `ORDER BY created_at DESC`
    // rather than returning an error or producing a SQL injection.
    let q = AdvancedEntityQuery {
        filter: None,
        limit: None,
        offset: None,
        sort_by: Some("name; DROP TABLE workflows--".to_string()),
        sort_direction: None,
    };
    let results = query_repo.query_entities(&def.entity_type, &q).await?;
    assert!(!results.is_empty());

    Ok(())
}

// ── empty result when no entities exist ──────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_query_empty_result_for_empty_entity_type() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_empty").await?;
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());

    let q = AdvancedEntityQuery {
        filter: None,
        limit: None,
        offset: None,
        sort_by: None,
        sort_direction: None,
    };
    let results = query_repo.query_entities(&def.entity_type, &q).await?;
    assert!(results.is_empty());

    Ok(())
}

// ── unknown entity type returns error ────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_query_unknown_entity_type_returns_error() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());

    let q = AdvancedEntityQuery {
        filter: None,
        limit: None,
        offset: None,
        sort_by: None,
        sort_direction: None,
    };
    let result = query_repo
        .query_entities("definitely_does_not_exist_xyz", &q)
        .await;
    assert!(result.is_err());

    Ok(())
}

// ── Some(empty HashMap) behaves the same as no filter ────────────────────────

#[tokio::test]
#[serial]
async fn test_query_empty_filter_map_returns_all() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_emptyfilter").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());

    for i in 0..2_i64 {
        entity_repo
            .create(&make_entity(&def, &format!("ef-{i}"), i))
            .await?;
    }

    // An empty filter map must not add a WHERE clause.
    let q = AdvancedEntityQuery {
        filter: Some(HashMap::new()),
        limit: None,
        offset: None,
        sort_by: None,
        sort_direction: None,
    };
    let results = query_repo.query_entities(&def.entity_type, &q).await?;
    assert_eq!(results.len(), 2);

    Ok(())
}

// ── unknown / malicious sort field is rejected by the allowlist ────────────────

#[tokio::test]
#[serial]
async fn test_query_unknown_sort_field_falls_back_safely() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_bad_sort").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());
    entity_repo.create(&make_entity(&def, "Alice", 10)).await?;
    entity_repo.create(&make_entity(&def, "Bob", 20)).await?;

    // An unknown column and an injection attempt must both fall back to the
    // default sort (no error, no arbitrary-column ordering, no injection).
    for bad in ["totally_unknown_column", "name); DROP TABLE x; --"] {
        let q = AdvancedEntityQuery {
            filter: None,
            limit: None,
            offset: None,
            sort_by: Some(bad.to_string()),
            sort_direction: Some("DESC".to_string()),
        };
        let results = query_repo.query_entities(&def.entity_type, &q).await?;
        assert_eq!(
            results.len(),
            2,
            "unknown sort `{bad}` should still return rows"
        );
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_query_valid_sort_field_orders_results() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_good_sort").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());
    entity_repo.create(&make_entity(&def, "Alice", 10)).await?;
    entity_repo.create(&make_entity(&def, "Bob", 30)).await?;
    entity_repo.create(&make_entity(&def, "Cara", 20)).await?;

    let q = AdvancedEntityQuery {
        filter: None,
        limit: None,
        offset: None,
        sort_by: Some("score".to_string()),
        sort_direction: Some("DESC".to_string()),
    };
    let results = query_repo.query_entities(&def.entity_type, &q).await?;
    let scores: Vec<i64> = results
        .iter()
        .filter_map(|e| {
            e.field_data
                .get("score")
                .and_then(serde_json::Value::as_i64)
        })
        .collect();
    assert_eq!(
        scores,
        vec![30, 20, 10],
        "valid sort field must order results"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_query_unknown_filter_field_is_ignored() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let def = setup_entity_type(&pool.pool, "qr_bad_filter").await?;
    let entity_repo = DynamicEntityRepository::new(pool.pool.clone());
    let query_repo = DynamicEntityQueryRepository::new(pool.pool.clone());
    entity_repo.create(&make_entity(&def, "Alice", 10)).await?;
    entity_repo.create(&make_entity(&def, "Bob", 20)).await?;

    // Filtering on a column that isn't in the entity definition is ignored
    // (no WHERE clause added, no error) rather than interpolated.
    let mut filter = HashMap::new();
    filter.insert("not_a_real_field".to_string(), serde_json::json!("x"));
    let q = AdvancedEntityQuery {
        filter: Some(filter),
        limit: None,
        offset: None,
        sort_by: None,
        sort_direction: None,
    };
    let results = query_repo.query_entities(&def.entity_type, &q).await?;
    assert_eq!(results.len(), 2, "unknown filter field should be ignored");

    Ok(())
}
