use sqlx::{PgPool};
use log::debug;

use crate::error::{Error, Result};

/// Create or update a PostgreSQL enum type from a list of values
/// This is useful for mapping Rust enums to PostgreSQL enum types
pub async fn create_or_update_enum(db: &PgPool, enum_name: &str, values: &[String]) -> Result<()> {
    // Check if the enum type already exists
    let enum_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM pg_type JOIN pg_namespace ON pg_type.typnamespace = pg_namespace.oid
            WHERE pg_type.typname = $1 AND pg_type.typtype = 'e'
        )"
    )
    .bind(format!("{}_enum", enum_name))
    .fetch_one(db)
    .await
    .map_err(Error::Database)?;
    
    if !enum_exists {
        // Create the enum type if it doesn't exist
        if values.is_empty() {
            // Create with a placeholder value if no values provided
            let query = format!(
                "CREATE TYPE {}_enum AS ENUM ('__placeholder__')",
                enum_name
            );
            
            sqlx::query(&query)
                .execute(db)
                .await
                .map_err(Error::Database)?;
        } else {
            // Create with the provided values
            let values_str = values
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect::<Vec<_>>()
                .join(", ");
                
            let query = format!(
                "CREATE TYPE {}_enum AS ENUM ({})",
                enum_name, values_str
            );
            
            sqlx::query(&query)
                .execute(db)
                .await
                .map_err(Error::Database)?;
        }
        
        debug!("Created enum type: {}_enum", enum_name);
    } else {
        // Check existing enum values and add new ones if needed
        for value in values {
            let value_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                    SELECT 1 FROM pg_enum 
                    JOIN pg_type ON pg_enum.enumtypid = pg_type.oid
                    WHERE pg_type.typname = $1 AND pg_enum.enumlabel = $2
                )"
            )
            .bind(format!("{}_enum", enum_name))
            .bind(value)
            .fetch_one(db)
            .await
            .map_err(Error::Database)?;
            
            if !value_exists {
                // Add the new value to the enum
                let query = format!(
                    "ALTER TYPE {}_enum ADD VALUE '{}'",
                    enum_name, value.replace("'", "''")
                );
                
                sqlx::query(&query)
                    .execute(db)
                    .await
                    .map_err(Error::Database)?;
                    
                debug!("Added value '{}' to enum type: {}_enum", value, enum_name);
            }
        }
    }
    
    Ok(())
} 