use r_data_core::api::admin::class_definitions::repository::ClassDefinitionRepository;
use r_data_core::entity::class::definition::ClassDefinition;
use r_data_core::entity::field::definition::FieldDefinition;
use r_data_core::entity::field::types::FieldType;
use r_data_core::entity::field::ui::UiSettings;
use sqlx::PgPool;
use std::env;
use log::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Starting field rename test");

    // Get database URL from environment or use default
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/rdata".to_string()
    });

    // Initialize database connection
    let db_pool = PgPool::connect(&database_url).await?;
    let repo = ClassDefinitionRepository::new(db_pool.clone());
    
    // Create a test class definition
    let mut class_def = ClassDefinition::new(
        "test_rename_field_fix".to_string(),
        "Test Rename Field Fix".to_string(),
        Some("Test class for field renaming".to_string()),
        Some("test".to_string()),
        false,
        Some("test".to_string()),
        vec![
            FieldDefinition {
                name: "original_field_name".to_string(),
                display_name: "Original Field Name".to_string(),
                description: Some("This field will be renamed".to_string()),
                field_type: FieldType::String,
                required: true,
                indexed: true,
                filterable: true,
                constraints: Default::default(),
                validation: Default::default(),
                ui_settings: UiSettings::default(),
                default_value: None,
            },
            FieldDefinition {
                name: "another_field".to_string(),
                display_name: "Another Field".to_string(),
                description: Some("This field won't change".to_string()),
                field_type: FieldType::Integer,
                required: false,
                indexed: false,
                filterable: true,
                constraints: Default::default(),
                validation: Default::default(),
                ui_settings: UiSettings::default(),
                default_value: None,
            }
        ]
    );

    // Create the class definition in the database
    info!("Creating test class definition");
    let uuid = repo.create(&class_def).await?;
    info!("Created class definition with UUID: {}", uuid);

    // Update the entity table structure
    info!("Updating entity table structure");
    repo.update_entity_table_for_class_definition(&class_def).await?;
    
    // Check the table columns before rename
    let table_name = class_def.get_table_name();
    info!("Table name: {}", table_name);
    
    let columns_before = repo.get_table_columns_with_types(&table_name).await?;
    info!("Columns before rename: {:?}", columns_before);
    
    // Now rename the field
    info!("Renaming field 'original_field_name' to 'renamed_field'");
    let fields = class_def.fields.iter_mut();
    for field in fields {
        if field.name == "original_field_name" {
            field.name = "renamed_field".to_string();
            field.display_name = "Renamed Field".to_string();
            break;
        }
    }
    
    // Update the class definition in the database
    info!("Updating class definition");
    repo.update(&uuid, &class_def).await?;
    
    // Update the entity table structure again
    info!("Updating entity table structure after rename");
    repo.update_entity_table_for_class_definition(&class_def).await?;
    
    // Check the table columns after rename
    let columns_after = repo.get_table_columns_with_types(&table_name).await?;
    info!("Columns after rename: {:?}", columns_after);
    
    // Verify that the old column is gone and the new one exists
    if columns_after.contains_key("original_field_name") {
        error!("FAIL: Original field name still exists in the table!");
    } else {
        info!("SUCCESS: Original field name was removed from the table");
    }
    
    if columns_after.contains_key("renamed_field") {
        info!("SUCCESS: New field name exists in the table");
    } else {
        error!("FAIL: New field name does not exist in the table!");
    }

    // Clean up
    info!("Cleaning up - deleting test class definition");
    repo.delete(&uuid).await?;
    
    info!("Test completed");
    Ok(())
} 