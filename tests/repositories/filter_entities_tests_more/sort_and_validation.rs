#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! `filter.rs::add_sort_and_pagination` invalid-direction fallback,
//! identifier validation errors surfaced through the public `filter_entities`
//! call, and combined filter+search (AND join).

use serde_json::json;
use std::collections::HashMap;

use r_data_core_core::error::Result;
use r_data_core_persistence::FilterEntitiesParams;
use r_data_core_persistence::{DynamicEntityRepository, DynamicEntityRepositoryTrait};
use r_data_core_test_support::{clear_test_db, setup_test_db};

use super::common::{create_filter_entity, create_filter_entity_definition, unique_entity_type};

#[tokio::test]
async fn test_sort_invalid_direction_defaults_to_desc() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("sort_invalid_dir");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "sort-1",
        HashMap::from([("age".to_string(), json!(20))]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "sort-2",
        HashMap::from([("age".to_string(), json!(40))]),
    )
    .await?;

    // "SIDEWAYS" is neither ASC nor DESC — must fall back to DESC.
    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_sort(Some(("age".to_string(), "SIDEWAYS".to_string()))),
        )
        .await?;

    assert_eq!(result.len(), 2);
    assert_eq!(
        result[0].field_data.get("age").unwrap().as_i64(),
        Some(40),
        "invalid direction falls back to DESC (highest age first)"
    );

    Ok(())
}

#[tokio::test]
async fn test_filter_unknown_field_returns_validation_error() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("bad_filter_field");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0).with_filters(Some(HashMap::from([(
                "not_a_real_field".to_string(),
                json!(1),
            )]))),
        )
        .await;

    assert!(
        result.is_err(),
        "unknown filter field must be rejected before hitting the database"
    );

    Ok(())
}

#[tokio::test]
async fn test_sort_unknown_field_returns_validation_error() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("bad_sort_field");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_sort(Some(("not_a_real_field".to_string(), "ASC".to_string()))),
        )
        .await;

    assert!(
        result.is_err(),
        "unknown sort field must be rejected before hitting the database"
    );

    Ok(())
}

#[tokio::test]
async fn test_filter_and_search_combine_with_and() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("filter_and_search");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "combo-1",
        HashMap::from([
            ("name".to_string(), json!("Alice Match")),
            ("age".to_string(), json!(20)),
        ]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "combo-2",
        HashMap::from([
            ("name".to_string(), json!("Alice Match")),
            ("age".to_string(), json!(40)),
        ]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "combo-3",
        HashMap::from([
            ("name".to_string(), json!("Bob NoMatch")),
            ("age".to_string(), json!(20)),
        ]),
    )
    .await?;

    // filters AND search: age = 20 AND name ILIKE %Alice%
    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!(20))])))
                .with_search(Some(("Alice".to_string(), vec!["name".to_string()]))),
        )
        .await?;

    assert_eq!(
        result.len(),
        1,
        "only the entity matching both the filter and the search term qualifies"
    );
    assert_eq!(result[0].field_data.get("entity_key").unwrap(), "combo-1");

    Ok(())
}
