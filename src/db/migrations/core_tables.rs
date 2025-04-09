use log::info;
use sqlx::{query, PgPool};

use crate::error::{Error, Result};

/// Create entities registry table
pub async fn create_entities_registry_table(pool: &PgPool) -> Result<()> {
    info!("Creating entities registry table...");

    // Create entity_registry table for storing metadata about entity types
    query(
        r#"
        CREATE TABLE IF NOT EXISTS entity_registry (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            class_name VARCHAR(100) NOT NULL UNIQUE,
            display_name VARCHAR(255) NOT NULL,
            table_name VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    Ok(())
}

/// Create entity versions table
pub async fn create_entity_versions_table(pool: &PgPool) -> Result<()> {
    info!("Creating entity versions table...");

    // Create entity_versions table for storing version history
    query(
        r#"
        CREATE TABLE IF NOT EXISTS entity_versions (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            entity_id UUID NOT NULL,
            entity_type VARCHAR(100) NOT NULL,
            version INT NOT NULL,
            data JSONB NOT NULL,
            created_by UUID,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            comment TEXT,
            UNIQUE(entity_id, version)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create indices for faster lookups
    query("CREATE INDEX IF NOT EXISTS idx_entity_versions_entity_id ON entity_versions(entity_id)")
        .execute(pool)
        .await
        .map_err(|e| Error::Database(e))?;

    query(
        "CREATE INDEX IF NOT EXISTS idx_entity_versions_entity_type ON entity_versions(entity_type)"
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    Ok(())
}

/// Create class definitions table
pub async fn create_class_definitions_table(pool: &PgPool) -> Result<()> {
    info!("Creating class definitions table...");

    // Create class_definitions table for storing entity class schemas
    query(
        r#"
        CREATE TABLE IF NOT EXISTS class_definitions (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            uuid UUID NOT NULL UNIQUE,
            path TEXT NOT NULL,
            class_name VARCHAR(100) NOT NULL UNIQUE,
            display_name VARCHAR(255) NOT NULL,
            description TEXT,
            group_name VARCHAR(100),
            allow_children BOOLEAN NOT NULL DEFAULT FALSE,
            icon VARCHAR(100),
            fields JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            created_by UUID,
            updated_by UUID,
            published BOOLEAN NOT NULL DEFAULT FALSE,
            version INTEGER NOT NULL DEFAULT 1,
            custom_fields JSONB NOT NULL DEFAULT '{}'
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create indices for faster lookups
    query("CREATE INDEX IF NOT EXISTS idx_class_definitions_uuid ON class_definitions(uuid)")
        .execute(pool)
        .await
        .map_err(|e| Error::Database(e))?;

    query("CREATE INDEX IF NOT EXISTS idx_class_definitions_path ON class_definitions(path)")
        .execute(pool)
        .await
        .map_err(|e| Error::Database(e))?;

    Ok(())
}

/// Create entities data table
pub async fn create_entities_data_table(pool: &PgPool) -> Result<()> {
    info!("Creating entities data table...");

    // Create entities_registry table for storing metadata about entity types
    query(
        r#"
        CREATE TABLE IF NOT EXISTS entities_registry (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            uuid UUID NOT NULL UNIQUE,
            path VARCHAR(255) NOT NULL,
            entity_type VARCHAR(50) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            created_by UUID,
            updated_by UUID,
            published BOOLEAN NOT NULL DEFAULT FALSE,
            version INTEGER NOT NULL DEFAULT 1,
            data JSONB NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    Ok(())
}
