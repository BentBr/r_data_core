#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::missing_errors_doc)]

//! Shared helpers for `filter_entities_tests_more`.
//!
//! An entity definition with numeric/boolean/float fields plus a raw-insert
//! helper, so sibling test files can exercise `filter.rs` operators, NULL
//! handling, path filters and type conversion without re-declaring
//! field-definition boilerplate.

use serde_json::{json, Value as JsonValue};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::DynamicEntityRepository;

fn field(name: &str, field_type: FieldType) -> FieldDefinition {
    FieldDefinition {
        name: name.to_string(),
        display_name: name.to_string(),
        field_type,
        required: false,
        description: None,
        filterable: true,
        unique: false,
        indexed: false,
        default_value: None,
        validation: FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: HashMap::new(),
    }
}

/// A unique entity type name so tests never collide with one another.
#[must_use]
pub fn unique_entity_type(base: &str) -> String {
    format!("{base}_{}", Uuid::now_v7().simple())
}

/// Create an entity definition with `name` (String), `age` (Integer),
/// `active` (Boolean), and `score` (Float) fields, then wait for the
/// generated table/view to become available.
///
/// # Panics
/// Panics if the row insert or view-creation query fails.
pub async fn create_filter_entity_definition(pool: &PgPool, entity_type: &str) {
    let entity_def = EntityDefinition {
        entity_type: entity_type.to_string(),
        display_name: format!("Test {entity_type}"),
        description: Some("filter test entity".to_string()),
        published: true,
        fields: vec![
            field("name", FieldType::String),
            field("age", FieldType::Integer),
            field("active", FieldType::Boolean),
            field("score", FieldType::Float),
        ],
        ..Default::default()
    };

    let create_query =
        "INSERT INTO entity_definitions (uuid, entity_type, display_name, description, field_definitions, created_at, created_by, published)
         VALUES ($1, $2, $3, $4, $5, NOW(), $6, $7) RETURNING uuid";

    let uuid = Uuid::now_v7();
    let created_by = Uuid::now_v7();

    sqlx::query(create_query)
        .bind(uuid)
        .bind(&entity_def.entity_type)
        .bind(&entity_def.display_name)
        .bind(&entity_def.description)
        .bind(json!(entity_def.fields))
        .bind(created_by)
        .bind(entity_def.published)
        .fetch_one(pool)
        .await
        .expect("failed to insert test entity definition");

    let trigger_sql = format!("SELECT create_entity_table_and_view('{entity_type}')");
    sqlx::query(&trigger_sql)
        .execute(pool)
        .await
        .expect("failed to trigger view creation");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}

/// Insert an entity directly, merging `overrides` on top of the required system fields.
///
/// Merges over `uuid`, `entity_key`, `created_by`. Bypasses `DynamicEntity::set`
/// validation so tests can supply raw values (including deliberately omitting
/// fields to produce NULL columns).
///
/// # Errors
/// Returns whatever error `DynamicEntityRepository::create` returns.
#[allow(clippy::implicit_hasher)] // test-only helper; generalizing over BuildHasher adds no value here
pub async fn create_filter_entity(
    pool: &PgPool,
    entity_type: &str,
    key: &str,
    overrides: HashMap<String, JsonValue>,
) -> Result<Uuid> {
    let uuid = Uuid::now_v7();
    let created_by = Uuid::now_v7();

    let mut field_data = HashMap::new();
    field_data.insert("uuid".to_string(), json!(uuid.to_string()));
    field_data.insert("entity_key".to_string(), json!(key));
    field_data.insert("created_by".to_string(), json!(created_by.to_string()));
    for (k, v) in overrides {
        field_data.insert(k, v);
    }

    let entity = DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data,
        definition: Arc::new(EntityDefinition::default()),
    };

    let repository = DynamicEntityRepository::new(pool.clone());
    repository.create(&entity).await
}
