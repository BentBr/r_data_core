#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! `filter.rs::convert_value_to_type` — string-encoded filter values coerced
//! to the field's real type (boolean/integer/float), as happens when filter
//! values arrive as query-string parameters.

use serde_json::json;
use std::collections::HashMap;

use r_data_core_core::error::Result;
use r_data_core_persistence::FilterEntitiesParams;
use r_data_core_persistence::{DynamicEntityRepository, DynamicEntityRepositoryTrait};
use r_data_core_test_support::{clear_test_db, setup_test_db};

use super::common::{create_filter_entity, create_filter_entity_definition, unique_entity_type};

#[tokio::test]
async fn test_filter_string_true_matches_boolean_field() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("bool_conv");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "bool-active",
        HashMap::from([("active".to_string(), json!(true))]),
    )
    .await?;
    create_filter_entity(
        &pool.pool,
        &entity_type,
        "bool-inactive",
        HashMap::from([("active".to_string(), json!(false))]),
    )
    .await?;

    for truthy in ["true", "1", "yes", "on", "TRUE"] {
        let result = repo
            .filter_entities(
                &entity_type,
                &FilterEntitiesParams::new(100, 0)
                    .with_filters(Some(HashMap::from([("active".to_string(), json!(truthy))]))),
            )
            .await?;
        assert_eq!(
            result.len(),
            1,
            "string {truthy:?} should convert to boolean true"
        );
        assert_eq!(
            result[0].field_data.get("entity_key").unwrap(),
            "bool-active"
        );
    }

    for falsy in ["false", "0", "no", "off"] {
        let result = repo
            .filter_entities(
                &entity_type,
                &FilterEntitiesParams::new(100, 0)
                    .with_filters(Some(HashMap::from([("active".to_string(), json!(falsy))]))),
            )
            .await?;
        assert_eq!(
            result.len(),
            1,
            "string {falsy:?} should convert to boolean false"
        );
        assert_eq!(
            result[0].field_data.get("entity_key").unwrap(),
            "bool-inactive"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_filter_string_integer_matches_integer_field() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("int_conv");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "int-conv-1",
        HashMap::from([("age".to_string(), json!(42))]),
    )
    .await?;

    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("age".to_string(), json!("42"))]))),
        )
        .await?;

    assert_eq!(result.len(), 1, "string \"42\" must convert to integer 42");

    Ok(())
}

#[tokio::test]
async fn test_filter_string_float_matches_float_field() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("float_conv");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "float-conv-1",
        HashMap::from([("score".to_string(), json!(3.5))]),
    )
    .await?;

    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0)
                .with_filters(Some(HashMap::from([("score".to_string(), json!("3.5"))]))),
        )
        .await?;

    assert_eq!(result.len(), 1, "string \"3.5\" must convert to float 3.5");

    Ok(())
}

#[tokio::test]
async fn test_filter_unparseable_string_against_integer_field_errors() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let entity_type = unique_entity_type("unparseable_conv");
    create_filter_entity_definition(&pool.pool, &entity_type).await;
    let repo = DynamicEntityRepository::new(pool.pool.clone());

    create_filter_entity(
        &pool.pool,
        &entity_type,
        "unparseable-1",
        HashMap::from([("age".to_string(), json!(42))]),
    )
    .await?;

    // A non-numeric string against an Integer field can't be converted by
    // `convert_value_to_type`, so it is kept as-is and bound as TEXT. Postgres
    // then rejects `integer = text` with no implicit cast, so the whole query
    // fails rather than returning an empty result set. This is a real (if
    // minor) rough edge in `filter.rs::convert_value_to_type` / callers should
    // be aware bad numeric-looking filters surface as a 500, not an empty
    // list — documented here rather than fixed (production code untouched).
    let result = repo
        .filter_entities(
            &entity_type,
            &FilterEntitiesParams::new(100, 0).with_filters(Some(HashMap::from([(
                "age".to_string(),
                json!("not-a-number"),
            )]))),
        )
        .await;

    assert!(
        result.is_err(),
        "an unparseable string against an Integer field surfaces as a database error"
    );

    Ok(())
}
