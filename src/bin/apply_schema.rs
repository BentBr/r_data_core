use clap::{App, Arg};
use log::{error, info};
use r_data_core::api::admin::class_definitions::repository::ClassDefinitionRepository;
use r_data_core::config::AppConfig;
use r_data_core::entity::class::repository_trait::ClassDefinitionRepositoryTrait;
use r_data_core::error::{Error, Result};
use sqlx::postgres::PgPoolOptions;
use std::process;
use uuid::Uuid;

async fn run() -> Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Parse command line arguments
    let matches = App::new("Apply Schema")
        .version("1.0")
        .author("R Data Core")
        .about("Applies database schemas for class definitions")
        .arg(
            Arg::with_name("uuid")
                .long("uuid")
                .short('u')
                .value_name("UUID")
                .help("UUID of the class definition to apply schema for. If not provided, all class definitions will be processed.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short('v')
                .help("Enable verbose output")
                .takes_value(false),
        )
        .get_matches();

    // Load configuration from environment
    let config = match AppConfig::from_env() {
        Ok(cfg) => {
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            eprintln!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    // Create database connection pool
    let db_pool = match PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.connection_string)
        .await
    {
        Ok(pool) => {
            info!("Database connection established");
            pool
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            eprintln!("Failed to connect to database: {}", e);
            process::exit(1);
        }
    };

    let repository = ClassDefinitionRepository::new(db_pool);

    // Determine if we're operating on a single class definition or all of them
    let uuid_str = matches.value_of("uuid");
    let verbose = matches.is_present("verbose");

    if let Some(uuid_str) = uuid_str {
        // Apply schema for a specific class definition
        match Uuid::parse_str(uuid_str) {
            Ok(uuid) => {
                println!("Applying schema for class definition with UUID: {}", uuid);

                match repository.get_by_uuid(&uuid).await {
                    Ok(Some(definition)) => {
                        println!(
                            "Found class definition: {} ({})",
                            definition.entity_type, definition.display_name
                        );

                        // Generate SQL schema for the entity type
                        let schema_sql = definition.generate_schema_sql();

                        if verbose {
                            println!("Generated SQL schema:\n{}", schema_sql);
                        }

                        match repository.apply_schema(&schema_sql).await {
                            Ok(_) => {
                                println!(
                                    "✅ Successfully applied schema for entity type: {}",
                                    definition.entity_type
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "❌ Failed to apply schema for entity type {}: {}",
                                    definition.entity_type, e
                                );
                                return Err(e);
                            }
                        }
                    }
                    Ok(None) => {
                        eprintln!("❌ Class definition with UUID {} not found", uuid);
                        return Err(Error::NotFound(format!(
                            "ClassDefinition with UUID {}",
                            uuid
                        )));
                    }
                    Err(e) => {
                        eprintln!(
                            "❌ Error fetching class definition with UUID {}: {}",
                            uuid, e
                        );
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Invalid UUID format: {}", e);
                process::exit(1);
            }
        }
    } else {
        // Apply schema for all class definitions
        println!("Applying schema for all class definitions...");

        match repository.list(1000, 0).await {
            Ok(definitions) => {
                if definitions.is_empty() {
                    println!("No class definitions found.");
                    return Ok(());
                }

                println!("Found {} class definitions", definitions.len());

                let mut success_count = 0;
                let mut failure_count = 0;

                for definition in &definitions {
                    println!(
                        "\nProcessing {}: {} ({})",
                        definition.uuid, definition.entity_type, definition.display_name
                    );

                    // Generate SQL schema for the entity type
                    let schema_sql = definition.generate_schema_sql();

                    if verbose {
                        println!("Generated SQL schema:\n{}", schema_sql);
                    }

                    match repository.apply_schema(&schema_sql).await {
                        Ok(_) => {
                            println!(
                                "✅ Successfully applied schema for entity type: {}",
                                definition.entity_type
                            );
                            success_count += 1;
                        }
                        Err(e) => {
                            eprintln!(
                                "❌ Failed to apply schema for entity type {}: {}",
                                definition.entity_type, e
                            );
                            failure_count += 1;
                        }
                    }
                }

                println!("\nSummary:");
                println!("Total processed: {}", definitions.len());
                println!("Successful: {}", success_count);
                println!("Failed: {}", failure_count);

                if failure_count > 0 {
                    eprintln!("\n❌ Some schemas failed to apply");
                    process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to list class definitions: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => {
            println!("\nOperation completed successfully!");
            process::exit(0);
        }
        Err(e) => {
            eprintln!("\nOperation failed: {}", e);
            process::exit(1);
        }
    }
}
