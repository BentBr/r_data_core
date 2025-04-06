use sqlx::{PgPool, query_as};
use log::{info, debug};

use crate::error::{Error, Result};
use crate::entity::ClassDefinition;

/// Refresh all class schemas in the database
pub async fn refresh_classes_schema(pool: &PgPool) -> Result<()> {
    info!("Refreshing class schemas...");
    
    // Get all class definitions from the database
    let rows = query_as::<_, (i64, String)>(
        "SELECT id, class_name FROM class_definitions"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(e))?;
    
    if rows.is_empty() {
        info!("No class definitions found, schema refresh skipped");
        return Ok(());
    }
    
    info!("Found {} class definitions to refresh", rows.len());
    
    // Load and apply each class definition
    for (id, class_name) in rows {
        debug!("Refreshing schema for class {}", class_name);
        
        // Load the class definition
        let class_def_row = query_as::<_, (serde_json::Value,)>(
            "SELECT fields FROM class_definitions WHERE id = $1"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::Database(e))?;
        
        // Try to deserialize the class definition
        let fields: Vec<crate::entity::field::FieldDefinition> = serde_json::from_value(class_def_row.0)
            .map_err(|e| Error::Serialization(e))?;
        
        // Create or update class definition
        let mut class_def = ClassDefinition::new(class_name.clone(), class_name.clone());
        class_def.fields = fields;
        
        // Apply the schema to the database
        class_def.apply_to_database(pool).await?;
        
        debug!("Schema refreshed for class {}", class_name);
    }
    
    info!("Schema refresh completed for all classes");
    Ok(())
} 