use chrono::Utc;
use r_data_core::api::admin::class_definitions::repository::ClassDefinitionRepository;
use r_data_core::config::{
    ApiConfig, AppConfig, CacheConfig, DatabaseConfig, LogConfig, WorkflowConfig,
};
use r_data_core::entity::field::{FieldDefinition, FieldType, FieldValidation};
use r_data_core::entity::ClassDefinition;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::{ContextV7, Uuid};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = AppConfig {
        environment: "development".to_string(),
        database: DatabaseConfig {
            connection_string: "postgres://postgres:postgres@localhost:5432/r_data_core"
                .to_string(),
            max_connections: 10,
            connection_timeout: 10,
        },
        api: ApiConfig {
            host: "localhost".to_string(),
            port: 8080,
            use_tls: false,
            jwt_secret: "test-secret".to_string(),
            jwt_expiration: 60 * 24, // 1 day
            enable_docs: true,
            cors_origins: vec!["*".to_string()],
        },
        cache: CacheConfig {
            enabled: true,
            ttl: 300,
            max_size: 10000,
        },
        workflow: WorkflowConfig {
            worker_threads: 4,
            default_timeout: 300,
            max_concurrent: 10,
        },
        log: LogConfig {
            level: "info".to_string(),
            file: None,
        },
    };

    // Connect to database
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .acquire_timeout(Duration::from_secs(config.database.connection_timeout))
        .connect(&config.database.connection_string)
        .await?;

    println!("Connected to database");

    // Create ClassDefinitionRepository
    let repo = ClassDefinitionRepository::new(pool.clone());

    // Create a test class definition
    let context = ContextV7::new();
    let ts = uuid::timestamp::Timestamp::now(&context);
    let mut test_class = ClassDefinition {
        uuid: Uuid::new_v7(ts),
        entity_type: "test_schema_update".to_string(),
        display_name: "Test Schema Update".to_string(),
        description: Some("A class for testing schema updates".to_string()),
        group_name: Some("Test".to_string()),
        allow_children: false,
        icon: None,
        fields: vec![
            FieldDefinition {
                name: "field1".to_string(),
                display_name: "Field 1".to_string(),
                field_type: FieldType::String,
                description: Some("A string field".to_string()),
                required: true,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: FieldValidation::default(),
                ui_settings: Default::default(),
                constraints: Default::default(),
            },
            FieldDefinition {
                name: "field2".to_string(),
                display_name: "Field 2".to_string(),
                field_type: FieldType::Integer,
                description: Some("An integer field".to_string()),
                required: false,
                indexed: true,
                filterable: true,
                default_value: None,
                validation: FieldValidation::default(),
                ui_settings: Default::default(),
                constraints: Default::default(),
            },
        ],
        schema: Default::default(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: None,
        updated_by: None,
        published: false,
        version: 1,
    };

    // Create the class definition and apply initial schema
    println!("Creating initial class definition with 2 fields");
    repo.create(&test_class).await?;
    repo.update_entity_table_for_class_definition(&test_class)
        .await?;
    println!("Initial schema applied");

    // Let's modify the class definition by:
    // 1. Adding a new field
    // 2. Removing field2
    // 3. Adding an index to field1
    println!("Modifying class definition - adding field3, removing field2");

    // First, get the class definition from the database to ensure we have the latest version
    let mut updated_class = repo.get_by_uuid(&test_class.uuid).await?;

    // Remove field2
    updated_class.fields.retain(|f| f.name != "field2");

    // Add field3
    updated_class.fields.push(FieldDefinition {
        name: "field3".to_string(),
        display_name: "Field 3".to_string(),
        field_type: FieldType::DateTime,
        description: Some("A datetime field".to_string()),
        required: false,
        indexed: true,
        filterable: true,
        default_value: None,
        validation: FieldValidation::default(),
        ui_settings: Default::default(),
        constraints: Default::default(),
    });

    // Update the class definition and apply schema changes
    repo.update(&updated_class.uuid, &updated_class).await?;
    println!("Updated class definition in database");

    // Apply schema changes
    repo.update_entity_table_for_class_definition(&updated_class)
        .await?;
    println!("Schema updated successfully");

    // Now, let's add a relation field to test _uuid column handling
    println!("Adding a relation field");
    updated_class.fields.push(FieldDefinition {
        name: "related".to_string(),
        display_name: "Related Entity".to_string(),
        field_type: FieldType::ManyToOne,
        description: Some("A relation to another entity".to_string()),
        required: false,
        indexed: true,
        filterable: true,
        default_value: None,
        validation: FieldValidation {
            target_class: Some("another_entity".to_string()),
            ..Default::default()
        },
        ui_settings: Default::default(),
        constraints: Default::default(),
    });

    // Update the class definition and apply schema changes
    repo.update(&updated_class.uuid, &updated_class).await?;
    println!("Added relation field to class definition");

    // Apply schema changes
    repo.update_entity_table_for_class_definition(&updated_class)
        .await?;
    println!("Schema updated for relation field");

    // Finally, clean up by dropping the table
    sqlx::query(&format!(
        "DROP TABLE IF EXISTS entity_{}",
        updated_class.entity_type
    ))
    .execute(&pool)
    .await?;
    println!("Test table dropped");

    // And delete the class definition
    repo.delete(&updated_class.uuid).await?;
    println!("Class definition deleted");

    println!("Test completed successfully!");
    Ok(())
}
