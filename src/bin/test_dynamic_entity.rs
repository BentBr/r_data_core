use dotenv::dotenv;
use log::{error, info};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use r_data_core::entity::dynamic_entity::{DynamicEntity, DynamicEntityRepository};
use r_data_core::entity::value::Value;
use r_data_core::entity::{
    ClassDefinition, FieldDefinition, FieldType, FieldValidation, UiSettings,
};
use r_data_core::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    info!("Testing dynamic entity functionality...");

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    // 1. Create a sample class definition
    let mut class_def = ClassDefinition::new("Product".to_string(), "Product".to_string());

    // Add sample fields
    let name_field = FieldDefinition {
        name: "name".to_string(),
        display_name: "Product Name".to_string(),
        field_type: FieldType::String,
        description: Some("The name of the product".to_string()),
        required: true,
        indexed: true,
        filterable: true,
        default_value: None,
        validation: FieldValidation {
            min_length: Some(3),
            max_length: Some(100),
            ..Default::default()
        },
        ui_settings: UiSettings::default(),
        constraints: HashMap::new(),
    };

    let price_field = FieldDefinition {
        name: "price".to_string(),
        display_name: "Price".to_string(),
        field_type: FieldType::Float,
        description: Some("The price of the product".to_string()),
        required: true,
        indexed: true,
        filterable: true,
        default_value: None,
        validation: FieldValidation {
            positive_only: Some(true),
            ..Default::default()
        },
        ui_settings: UiSettings::default(),
        constraints: HashMap::new(),
    };

    let in_stock_field = FieldDefinition {
        name: "in_stock".to_string(),
        display_name: "In Stock".to_string(),
        field_type: FieldType::Boolean,
        description: Some("Whether the product is in stock".to_string()),
        required: true,
        indexed: true,
        filterable: true,
        default_value: Some(serde_json::Value::Bool(true)),
        validation: FieldValidation::default(),
        ui_settings: UiSettings::default(),
        constraints: HashMap::new(),
    };

    class_def
        .add_field(name_field)
        .expect("Failed to add name field");
    class_def
        .add_field(price_field)
        .expect("Failed to add price field");
    class_def
        .add_field(in_stock_field)
        .expect("Failed to add in_stock field");

    // Apply the class definition to the database
    class_def.apply_to_database(&pool).await?;

    // Create a repository for the dynamic entity
    let repository = DynamicEntityRepository::new(
        pool.clone(),
        "Product".to_string(),
        Some(Arc::new(class_def.clone())),
    );

    // Create a new dynamic entity
    let mut entity = DynamicEntity::new("Product".to_string(), Some(Arc::new(class_def.clone())));

    // Set field values
    entity.set("name", "Test Product")?;
    entity.set("price", 99.99)?;
    entity.set("in_stock", true)?;

    // Save the entity to the database
    let uuid = repository.create(&entity).await?;
    info!("Created entity with UUID: {}", uuid);

    // Retrieve the entity
    let retrieved_entity = repository.get(uuid).await?;
    info!("Retrieved entity: {:?}", retrieved_entity);

    // Check field values
    let name: String = retrieved_entity.get("name")?;
    let price: f64 = retrieved_entity.get("price")?;
    let in_stock: bool = retrieved_entity.get("in_stock")?;

    info!("Name: {}, Price: {}, In Stock: {}", name, price, in_stock);

    // Update the entity
    let mut updated_entity = retrieved_entity;
    updated_entity.set("price", 89.99)?;
    repository.update(&updated_entity).await?;
    info!("Updated entity");

    // Retrieve and check the updated entity
    let updated_retrieved = repository.get(uuid).await?;
    let updated_price: f64 = updated_retrieved.get("price")?;
    info!("Updated price: {}", updated_price);

    // List entities
    let entities = repository.list(None, Some(10), Some(0)).await?;
    info!("Found {} entities", entities.len());

    // List entities with filter
    let mut filters = HashMap::new();
    filters.insert(
        "name".to_string(),
        Value::String("Test Product".to_string()),
    );

    let filtered_entities = repository.list(Some(filters), Some(10), Some(0)).await?;
    info!("Found {} entities with filter", filtered_entities.len());

    // Delete the entity
    repository.delete(&uuid).await?;
    info!("Deleted entity");

    info!("Dynamic entity test completed successfully");
    Ok(())
}
