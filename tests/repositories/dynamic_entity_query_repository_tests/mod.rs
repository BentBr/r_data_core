#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod basic;
pub mod edge_cases;

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use r_data_core_core::error::Result;
use r_data_core_core::{
    entity_definition::definition::EntityDefinition, field::definition::FieldDefinition,
    field::types::FieldType,
};
use r_data_core_persistence::EntityDefinitionRepository;
use r_data_core_services::EntityDefinitionService;
use r_data_core_test_support::unique_entity_type;

/// Create a unique entity definition with `name` (String) and `score` (Integer)
/// fields, wait for the backing view DDL, and return the finalised definition.
///
/// # Errors
/// Returns an error if the entity definition cannot be created or retrieved.
pub async fn setup_entity_type(pool: &sqlx::PgPool, base: &str) -> Result<EntityDefinition> {
    let entity_type = unique_entity_type(base);

    let entity_def = EntityDefinition {
        uuid: Uuid::nil(),
        entity_type: entity_type.clone(),
        display_name: format!("Test {entity_type}"),
        description: Some(format!("Description for {entity_type}")),
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![
            FieldDefinition {
                name: "name".to_string(),
                display_name: "Name".to_string(),
                description: Some("Name field".to_string()),
                field_type: FieldType::String,
                required: true,
                indexed: true,
                filterable: true,
                unique: false,
                default_value: None,
                validation: r_data_core_core::field::FieldValidation::default(),
                ui_settings: r_data_core_core::field::ui::UiSettings::default(),
                constraints: HashMap::new(),
            },
            FieldDefinition {
                name: "score".to_string(),
                display_name: "Score".to_string(),
                description: Some("Score field".to_string()),
                field_type: FieldType::Integer,
                required: false,
                indexed: false,
                filterable: true,
                unique: false,
                default_value: None,
                validation: r_data_core_core::field::FieldValidation::default(),
                ui_settings: r_data_core_core::field::ui::UiSettings::default(),
                constraints: HashMap::new(),
            },
        ],
        schema: r_data_core_core::entity_definition::schema::Schema::default(),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::now_v7(),
        updated_by: None,
        published: true,
        version: 1,
    };

    let def_repo = EntityDefinitionRepository::new(pool.clone());
    let def_service = EntityDefinitionService::new_without_cache(Arc::new(def_repo));
    def_service.create_entity_definition(&entity_def).await?;

    // Allow the view DDL to execute before we query it.
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    def_service
        .get_entity_definition_by_entity_type(&entity_type)
        .await
}

/// Build a minimal `DynamicEntity` for insertion.
#[must_use]
pub fn make_entity(
    def: &EntityDefinition,
    name: &str,
    score: i64,
) -> r_data_core_core::DynamicEntity {
    let mut field_data = HashMap::new();
    field_data.insert("entity_key".to_string(), json!(Uuid::now_v7().to_string()));
    field_data.insert("name".to_string(), json!(name));
    field_data.insert("score".to_string(), json!(score));
    field_data.insert("created_by".to_string(), json!(Uuid::now_v7().to_string()));

    r_data_core_core::DynamicEntity {
        entity_type: def.entity_type.clone(),
        field_data,
        definition: Arc::new(def.clone()),
    }
}
