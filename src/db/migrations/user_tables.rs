use log::info;
use sqlx::{query, PgPool};

use crate::error::{Error, Result};

/// Create admin users table
pub async fn create_admin_users_table(pool: &PgPool) -> Result<()> {
    info!("Creating admin users table...");

    // Create admin users table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS admin_users (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            email VARCHAR(255) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            first_name VARCHAR(100),
            last_name VARCHAR(100),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            active BOOLEAN NOT NULL DEFAULT TRUE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    Ok(())
}

/// Create permission schemes table
pub async fn create_permission_schemes_table(pool: &PgPool) -> Result<()> {
    info!("Creating permission schemes table...");

    // Create roles table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS roles (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            name VARCHAR(100) NOT NULL UNIQUE,
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create permissions table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS permissions (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            name VARCHAR(100) NOT NULL UNIQUE,
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create user_roles junction table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS user_roles (
            user_id UUID NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
            role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (user_id, role_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create role_permissions junction table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS role_permissions (
            role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
            permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (role_id, permission_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    Ok(())
}

/// Create API keys table
pub async fn create_api_keys_table(pool: &PgPool) -> Result<()> {
    info!("Creating API keys table...");

    // Create api_keys table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS api_keys (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            user_id UUID NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
            name VARCHAR(255) NOT NULL,
            prefix VARCHAR(16) NOT NULL,
            key_hash VARCHAR(255) NOT NULL,
            expires_at TIMESTAMPTZ,
            active BOOLEAN NOT NULL DEFAULT TRUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_used_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create index on key_hash for fast lookup
    query("CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash)")
        .execute(pool)
        .await
        .map_err(|e| Error::Database(e))?;

    // Create index on prefix for quick lookups during authentication
    query("CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(prefix)")
        .execute(pool)
        .await
        .map_err(|e| Error::Database(e))?;

    // Create index on user_id to efficiently list keys by user
    query("CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id)")
        .execute(pool)
        .await
        .map_err(|e| Error::Database(e))?;

    Ok(())
}
