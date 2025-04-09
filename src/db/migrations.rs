use crate::error::{Error, Result};
use log::info;
use sqlx::{postgres::PgPool, query, query_as, FromRow};
use std::collections::HashSet;

mod core_tables;
pub mod enum_types;
pub mod schema;
mod user_tables;
mod workflow_tables;

pub use core_tables::*;
pub use enum_types::create_or_update_enum;
pub use user_tables::*;
pub use workflow_tables::*;

// Export key functions for use in other modules
pub use schema::refresh_classes_schema;

/// Runs all database migrations in the correct order
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    info!("Running database migrations...");

    // Create the migrations table if it doesn't exist
    query(
        r#"
        CREATE TABLE IF NOT EXISTS migrations (
            uuid SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL UNIQUE,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // List of migrations to apply in order
    type MigrationFn = for<'a> fn(
        &'a PgPool,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<()>> + 'a>,
    >;

    let migration_fns: Vec<(&str, MigrationFn)> = vec![
        ("001_core_tables", |pool| Box::pin(apply_core_tables(pool))),
        ("002_user_tables", |pool| Box::pin(apply_user_tables(pool))),
        ("003_workflow_tables", |pool| {
            Box::pin(apply_workflow_tables(pool))
        }),
        ("004_refresh_schema", |pool| {
            Box::pin(schema::refresh_classes_schema(pool))
        }),
    ];

    // Check which migrations have already been applied
    let applied_migrations: Vec<String> =
        query_as::<_, AppliedMigration>("SELECT name FROM migrations")
            .fetch_all(pool)
            .await
            .map_err(|e| Error::Database(e))?
            .into_iter()
            .map(|m| m.name)
            .collect();

    let applied_set: HashSet<&String> = applied_migrations.iter().collect();

    // Apply each migration that hasn't been applied yet
    for (name, migration_fn) in migration_fns {
        if !applied_set.contains(&name.to_string()) {
            info!("Applying migration: {}", name);
            migration_fn(pool).await?;

            // Record that this migration has been applied
            query("INSERT INTO migrations (name) VALUES ($1)")
                .bind(name)
                .execute(pool)
                .await
                .map_err(|e| Error::Database(e))?;

            info!("Migration {} applied successfully", name);
        } else {
            info!("Migration {} already applied, skipping", name);
        }
    }

    info!("All migrations completed successfully");
    Ok(())
}

#[derive(FromRow)]
struct AppliedMigration {
    name: String,
}

/// Apply all core tables
async fn apply_core_tables(pool: &PgPool) -> Result<()> {
    create_entities_registry_table(pool).await?;
    create_entity_versions_table(pool).await?;
    create_class_definitions_table(pool).await?;
    create_entities_data_table(pool).await?;
    Ok(())
}

/// Apply all user-related tables
async fn apply_user_tables(pool: &PgPool) -> Result<()> {
    create_admin_users_table(pool).await?;
    create_permission_schemes_table(pool).await?;
    create_api_keys_table(pool).await?;
    Ok(())
}

/// Apply all workflow-related tables
async fn apply_workflow_tables(pool: &PgPool) -> Result<()> {
    create_notifications_table(pool).await?;
    create_workflow_definitions_table(pool).await?;
    create_workflows_table(pool).await?;
    Ok(())
}
