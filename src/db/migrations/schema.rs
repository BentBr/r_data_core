use log::{debug, info};
use serde_json;
use sqlx::{query_as, PgPool};
use uuid::Uuid;

use crate::entity::ClassDefinition;
use crate::entity::FieldType;
use crate::error::{Error, Result};

/// Refresh all class schemas in the database
pub async fn refresh_classes_schema(pool: &PgPool) -> Result<()> {
    info!("Refreshing class schemas...");

    // Get all class definitions from the database
    let rows = query_as::<_, (Uuid, String)>("SELECT uuid, class_name FROM class_definitions")
        .fetch_all(pool)
        .await
        .map_err(|e| Error::Database(e))?;

    if rows.is_empty() {
        info!("No class definitions found, schema refresh skipped");
        return Ok(());
    }

    info!("Found {} class definitions to refresh", rows.len());

    // Load and apply each class definition
    for (uuid, class_name) in rows {
        debug!("Refreshing schema for class {}", class_name);

        // Load the class definition
        let class_def_row = query_as::<_, (serde_json::Value,)>(
            "SELECT fields FROM class_definitions WHERE uuid = $1",
        )
        .bind(uuid)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::Database(e))?;

        // Try to deserialize the class definition
        let fields: Vec<crate::entity::field::FieldDefinition> =
            serde_json::from_value(class_def_row.0).map_err(|e| Error::Serialization(e))?;

        // Create or update class definition
        let mut class_def = ClassDefinition::new(
            class_name.clone(),
            serde_json::json!({
                "type": "object",
                "properties": fields.iter().map(|f| {
                    (f.name.clone(), serde_json::json!({
                        "type": match f.field_type {
                            FieldType::String | FieldType::Text | FieldType::Wysiwyg => "string",
                            FieldType::Integer => "number",
                            FieldType::Float => "number",
                            FieldType::Boolean => "boolean",
                            FieldType::Select => "string",
                            FieldType::MultiSelect => "array",
                            FieldType::DateTime | FieldType::Date => "string",
                            FieldType::Object => "object",
                            FieldType::Array => "array",
                            FieldType::UUID => "string",
                            FieldType::ManyToOne | FieldType::ManyToMany => "string",
                            FieldType::Image | FieldType::File => "string",
                        },
                        "title": f.name.clone(),
                        "description": f.description.clone(),
                    }))
                }).collect::<serde_json::Map<String, serde_json::Value>>(),
                "required": fields.iter().filter(|f| f.required).map(|f| f.name.clone()).collect::<Vec<String>>(),
            }),
        );

        // Apply the schema to the database
        class_def.apply_to_database(pool).await?;

        debug!("Schema refreshed for class {}", class_name);
    }

    info!("Schema refresh completed for all classes");
    Ok(())
}
