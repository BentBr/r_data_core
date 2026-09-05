#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! `filter.rs::add_filter_condition` — comparison and IN/NOT IN operators
//! supplied via `FilterEntitiesParams::with_filter_operators`.

use serde_json::json;
use std::collections::HashMap;

use r_data_core_core::error::Result;
use r_data_core_persistence::FilterEntitiesParams;
use r_data_core_persistence::{DynamicEntityRepository, DynamicEntityRepositoryTrait};
use r_data_core_test_support::{clear_test_db, setup_test_db};

use super::common::{create_filter_entity, create_filter_entity_definition, unique_entity_type};

#[tokio::test]
async fn test_filter_greater_and_less_than() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("op_cmp");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    for (i, age) in [20, 30, 40].into_iter().enumerate() {
        create_filter_entity(
            &pool.pool,
            &entity_type,
            &format!("cmp-{i}"),
            HashMap::from([("age".to_string(), json!(age))]),
        )
        .await?;
    }

    let gt = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!(25))])))
                .with_filter_operators(Some(HashMap::from([("age".to_string(), ">".to_string())]))),
        )
        .await?;
    assert_eq!(gt.len(), 2, "ages 30 and 40 are > 25");

    let lt = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!(25))])))
                .with_filter_operators(Some(HashMap::from([("age".to_string(), "<".to_string())]))),
        )
        .await?;
    assert_eq!(lt.len(), 1, "only age 20 is < 25");

    let gte = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!(30))])))
                .with_filter_operators(Some(HashMap::from([(
                    "age".to_string(),
                    ">=".to_string(),
                )]))),
        )
        .await?;
    assert_eq!(gte.len(), 2, "ages 30 and 40 are >= 30");

    let lte = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!(30))])))
                .with_filter_operators(Some(HashMap::from([(
                    "age".to_string(),
                    "<=".to_string(),
                )]))),
        )
        .await?;
    assert_eq!(lte.len(), 2, "ages 20 and 30 are <= 30");

    Ok(())
}

#[tokio::test]
async fn test_filter_in_and_not_in_with_values() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("op_in");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    for (i, age) in [20, 30, 40].into_iter().enumerate() {
        create_filter_entity(
            &pool.pool,
            &entity_type,
            &format!("in-{i}"),
            HashMap::from([("age".to_string(), json!(age))]),
        )
        .await?;
    }

    let in_result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!([20, 40]))])))
                .with_filter_operators(Some(HashMap::from([(
                    "age".to_string(),
                    "IN".to_string(),
                )]))),
        )
        .await?;
    assert_eq!(in_result.len(), 2, "IN [20, 40] matches two entities");

    let not_in_result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!([20, 40]))])))
                .with_filter_operators(Some(HashMap::from([(
                    "age".to_string(),
                    "NOT IN".to_string(),
                )]))),
        )
        .await?;
    assert_eq!(not_in_result.len(), 1, "NOT IN [20, 40] leaves only age 30");

    Ok(())
}

#[tokio::test]
async fn test_filter_in_empty_array_matches_nothing() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("op_in_empty");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "empty-in-1",
        HashMap::from([("age".to_string(), json!(20))]),
    )
    .await?;

    let empty_in: Vec<serde_json::Value> = vec![];
    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!(empty_in))])))
                .with_filter_operators(Some(HashMap::from([(
                    "age".to_string(),
                    "IN".to_string(),
                )]))),
        )
        .await?;
    assert!(result.is_empty(), "IN [] must match nothing");

    Ok(())
}

#[tokio::test]
async fn test_filter_not_in_empty_array_matches_everything() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("op_not_in_empty");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "empty-not-in-1",
        HashMap::from([("age".to_string(), json!(20))]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "empty-not-in-2",
        HashMap::from([("age".to_string(), json!(30))]),
    )
    .await?;

    let empty_in: Vec<serde_json::Value> = vec![];
    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!(empty_in))])))
                .with_filter_operators(Some(HashMap::from([(
                    "age".to_string(),
                    "NOT IN".to_string(),
                )]))),
        )
        .await?;
    assert_eq!(result.len(), 2, "NOT IN [] must match everything");

    Ok(())
}

#[tokio::test]
async fn test_filter_in_with_scalar_value_treated_as_single() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("op_in_scalar");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "in-scalar-1",
        HashMap::from([("age".to_string(), json!(20))]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "in-scalar-2",
        HashMap::from([("age".to_string(), json!(30))]),
    )
    .await?;

    // A non-array value with an IN operator must fall back to single-value
    // comparison rather than erroring.
    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!(20))])))
                .with_filter_operators(Some(HashMap::from([(
                    "age".to_string(),
                    "IN".to_string(),
                )]))),
        )
        .await?;
    assert_eq!(result.len(), 1, "scalar IN behaves like equality");

    Ok(())
}
