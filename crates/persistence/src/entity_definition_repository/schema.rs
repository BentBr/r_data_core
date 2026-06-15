use super::EntityDefinitionRepository;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core_core::error::Error;
use r_data_core_core::error::Result;
use std::collections::HashSet;

/// Apply schema SQL to database
///
/// # Errors
/// Returns an error if any SQL statement fails to execute
pub async fn apply_schema(repo: &EntityDefinitionRepository, schema_sql: &str) -> Result<()> {
    // Split the SQL into individual statements and execute each one separately
    let statements: Vec<&str> = schema_sql
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    for statement in statements {
        if !statement.trim().is_empty() {
            log::debug!("Executing SQL statement: {statement}");
            sqlx::query(statement)
                .execute(&repo.db_pool)
                .await
                .map_err(Error::Database)?;
        }
    }
    Ok(())
}

/// Update entity table and view for entity definition
///
/// # Errors
/// Returns an error if the schema SQL generation or execution fails
pub async fn update_entity_view_for_entity_definition(
    repo: &EntityDefinitionRepository,
    entity_definition: &EntityDefinition,
) -> Result<()> {
    log::info!(
        "Updating entity table and view for entity type: {}",
        entity_definition.entity_type
    );

    // Generate the complete schema SQL including indexes
    let schema_sql = entity_definition.generate_schema_sql();

    log::debug!("Generated schema SQL: {schema_sql}");

    // Apply the schema using the Rust-generated SQL
    apply_schema(repo, &schema_sql).await?;

    // Clear the prepared statement cache to avoid "cached plan must not change result type" errors
    // This is necessary because the view structure may have changed.
    // DISCARD PLANS clears all cached plans for the current session
    log::debug!("Clearing prepared statement cache after view update");
    sqlx::query("DISCARD PLANS")
        .execute(&repo.db_pool)
        .await
        .map_err(Error::Database)?;

    log::info!(
        "Successfully created/updated table and view for entity type {}",
        entity_definition.entity_type
    );

    Ok(())
}

/// Cleanup unused entity tables
///
/// # Errors
/// Returns an error if the database queries fail
pub async fn cleanup_unused_entity_view(repo: &EntityDefinitionRepository) -> Result<()> {
    // Get all entity definitions
    let entity_definitions = repo.list(1000, 0).await?;

    // Get all tables starting with "entity_"
    let tables = sqlx::query!(
        "
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = current_schema()
        AND table_name LIKE 'entity_%'
        "
    )
    .fetch_all(&repo.db_pool)
    .await
    .map_err(Error::Database)?;

    // Identify tables without a corresponding entity definition
    let defined_tables: HashSet<String> = entity_definitions
        .iter()
        .map(r_data_core_core::entity_definition::definition::EntityDefinition::get_table_name)
        .collect();

    for row in tables {
        if let Some(table_name) = row.table_name {
            if !defined_tables.contains(&table_name) {
                // Table has no corresponding entity definition, drop it
                log::info!("Dropping orphaned entity table: {table_name}");
                let drop_sql = format!("DROP TABLE IF EXISTS {table_name} CASCADE");

                sqlx::query(&drop_sql)
                    .execute(&repo.db_pool)
                    .await
                    .map_err(Error::Database)?;
            }
        }
    }

    Ok(())
}
