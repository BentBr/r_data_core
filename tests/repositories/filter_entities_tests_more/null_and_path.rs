#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! `filter.rs::add_filter_condition` — NULL-value filters, the synthetic
//! `path_prefix`/`path_equals` keys, direct `path` equality, and operator
//! sanitization against injection attempts.

use serde_json::json;
use std::collections::HashMap;

use r_data_core_core::error::Result;
use r_data_core_persistence::FilterEntitiesParams;
use r_data_core_persistence::{DynamicEntityRepository, DynamicEntityRepositoryTrait};
use r_data_core_test_support::{clear_test_db, setup_test_db};

use super::common::{create_filter_entity, create_filter_entity_definition, unique_entity_type};

#[tokio::test]
async fn test_filter_null_value_matches_missing_field() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("null_filter");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    // One entity with age set, one without (column stays NULL).
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "null-has-age",
        HashMap::from([("age".to_string(), json!(20))]),
    )
    .await?;
    create_filter_entity(&pool.pool, &entity_type, "null-no-age", HashMap::new()).await?;

    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0).with_filters(Some(HashMap::from([(
                "age".to_string(),
                serde_json::Value::Null,
            )]))),
        )
        .await?;

    assert_eq!(result.len(), 1, "only the entity with a NULL age matches");
    assert_eq!(
        result[0].field_data.get("entity_key").unwrap(),
        "null-no-age"
    );

    Ok(())
}

#[tokio::test]
async fn test_filter_path_prefix_matches_descendants() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("path_prefix");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "under-foo-1",
        HashMap::from([("path".to_string(), json!("/foo/child1"))]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "under-foo-2",
        HashMap::from([("path".to_string(), json!("/foo/child2"))]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "under-bar",
        HashMap::from([("path".to_string(), json!("/bar/child1"))]),
    )
    .await?;

    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0).with_filters(Some(HashMap::from([(
                "path_prefix".to_string(),
                json!("/foo"),
            )]))),
        )
        .await?;

    assert_eq!(result.len(), 2, "only children of /foo match the prefix");

    Ok(())
}

#[tokio::test]
async fn test_filter_path_equals_and_direct_path_field() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("path_equals");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "exact-foo",
        HashMap::from([("path".to_string(), json!("/foo"))]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "exact-foobar",
        HashMap::from([("path".to_string(), json!("/foobar"))]),
    )
    .await?;

    let via_path_equals = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0).with_filters(Some(HashMap::from([(
                "path_equals".to_string(),
                json!("/foo"),
            )]))),
        )
        .await?;
    assert_eq!(
        via_path_equals.len(),
        1,
        "path_equals must not prefix-match"
    );

    let via_path_field = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("path".to_string(), json!("/foo"))]))),
        )
        .await?;
    assert_eq!(
        via_path_field.len(),
        1,
        "direct 'path' filter also exact-matches"
    );

    Ok(())
}

#[tokio::test]
async fn test_filter_invalid_operator_falls_back_to_equality() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("bad_operator");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "bad-op-1",
        HashMap::from([("age".to_string(), json!(20))]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "bad-op-2",
        HashMap::from([("age".to_string(), json!(30))]),
    )
    .await?;

    // An operator outside the whitelist must be sanitized down to "=" rather
    // than reaching the query verbatim.
    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!(20))])))
                .with_filter_operators(Some(HashMap::from([(
                    "age".to_string(),
                    "; DROP TABLE admin_users; --".to_string(),
                )]))),
        )
        .await?;

    assert_eq!(result.len(), 1, "invalid operator behaves as equality");
    assert_eq!(result[0].field_data.get("entity_key").unwrap(), "bad-op-1");

    Ok(())
}
