use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sqlx::{PgPool, FromRow, postgres::PgRow, Row};
use log::debug;
use utoipa::ToSchema;

use crate::error::{Error, Result};
use crate::entity::AbstractRDataEntity;
use crate::entity::field::{FieldDefinition, FieldType, OptionsSource};
use crate::db::create_or_update_enum;

/// Class definition for a custom RDataEntity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClassDefinition {
    /// Base entity properties
    #[serde(flatten)]
    pub base: AbstractRDataEntity,
    
    /// Unique class name (used in code/API)
    pub class_name: String,
    
    /// Display name for admin UI
    pub display_name: String,
    
    /// Class description
    pub description: Option<String>,
    
    /// Group name for organizing classes
    pub group: Option<String>,
    
    /// Whether instances can have children
    pub allow_children: bool,
    
    /// Icon for admin UI
    pub icon: Option<String>,
    
    /// Field definitions for this class
    pub fields: Vec<FieldDefinition>,
}

// Implement FromRow for ClassDefinition
impl<'r> FromRow<'r, PgRow> for ClassDefinition {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        // Extract the base entity data
        let id = row.try_get::<i64, _>("id")?;
        let uuid = row.try_get::<String, _>("uuid")?;
        let path = row.try_get::<String, _>("path")?;
        let created_at = row.try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")?;
        let updated_at = row.try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("updated_at")?;
        let created_by = row.try_get::<Option<i64>, _>("created_by").unwrap_or(None);
        let updated_by = row.try_get::<Option<i64>, _>("updated_by").unwrap_or(None);
        let published = row.try_get::<bool, _>("published").unwrap_or(false);
        let version = row.try_get::<i32, _>("version").unwrap_or(1);
        
        // Extract class definition specific fields
        let class_name = row.try_get::<String, _>("class_name")?;
        let display_name = row.try_get::<String, _>("display_name")?;
        let description = row.try_get::<Option<String>, _>("description").unwrap_or(None);
        let group = row.try_get::<Option<String>, _>("group_name").unwrap_or(None);
        let allow_children = row.try_get::<bool, _>("allow_children").unwrap_or(false);
        let icon = row.try_get::<Option<String>, _>("icon").unwrap_or(None);
        
        // Fields are stored as JSON
        let fields_json = row.try_get::<serde_json::Value, _>("fields")
            .unwrap_or(serde_json::json!([]));
            
        let fields: Vec<FieldDefinition> = serde_json::from_value(fields_json)
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "fields".to_string(),
                source: Box::new(e)
            })?;
        
        // Get custom fields as JSON Value first
        let custom_fields_json = row.try_get::<serde_json::Value, _>("custom_fields")
            .unwrap_or(serde_json::json!({}));
            
        // Convert to HashMap
        let custom_fields = serde_json::from_value(custom_fields_json)
            .unwrap_or_else(|_| HashMap::new());
        
        // Build and return the entity
        let base = AbstractRDataEntity {
            id: Some(id),
            uuid: uuid::Uuid::parse_str(&uuid).map_err(|e| sqlx::Error::ColumnDecode {
                index: "uuid".to_string(),
                source: Box::new(e)
            })?,
            path,
            created_at,
            updated_at,
            created_by,
            updated_by,
            published,
            version,
            custom_fields,
        };
        
        Ok(ClassDefinition {
            base,
            class_name,
            display_name,
            description,
            group,
            allow_children,
            icon,
            fields,
        })
    }
}

impl ClassDefinition {
    /// Create a new class definition
    pub fn new(class_name: String, display_name: String) -> Self {
        Self {
            base: AbstractRDataEntity::new("/classes".to_string()),
            class_name,
            display_name,
            description: None,
            group: None,
            allow_children: false,
            icon: None,
            fields: Vec::new(),
        }
    }
    
    /// Add a field to the class definition
    pub fn add_field(&mut self, field: FieldDefinition) -> Result<()> {
        // Check if field with same name already exists
        if self.fields.iter().any(|f| f.name == field.name) {
            return Err(Error::Entity(format!("Field with name '{}' already exists", field.name)));
        }
        
        self.fields.push(field);
        Ok(())
    }
    
    /// Get a field definition by name
    pub fn get_field(&self, name: &str) -> Option<&FieldDefinition> {
        self.fields.iter().find(|f| f.name == name)
    }
    
    /// Generate the SQL table name for this entity type
    pub fn get_table_name(&self) -> String {
        format!("entity_{}", self.class_name.to_lowercase())
    }
    
    /// Apply the class definition to the database 
    pub async fn apply_to_database(&self, db: &PgPool) -> Result<()> {
        // 1. Generate SQL schema and execute
        let sql = self.generate_sql_schema();
        debug!("Applying SQL schema for class {}: {}", self.class_name, sql);
        
        // Execute the SQL schema
        sqlx::query(&sql)
            .execute(db)
            .await
            .map_err(Error::Database)?;
        
        // Create enum types for select fields if needed
        for field in &self.fields {
            if matches!(field.field_type, FieldType::Select | FieldType::MultiSelect) {
                if let Some(options_source) = &field.validation.options_source {
                    match options_source {
                        OptionsSource::Enum { enum_name } => {
                            // For enum type, we need to extract values from somewhere
                            // If no fixed options provided, we create an empty enum that can be populated later
                            let values = Vec::new();
                            crate::db::create_or_update_enum(db, enum_name, &values).await?;
                        },
                        OptionsSource::Fixed { options } => {
                            // For fixed options, create an enum with the option values
                            // Only if the field needs an enum - not all select fields do
                            if field.constraints.get("use_enum").map_or(false, |v| v.as_bool().unwrap_or(false)) {
                                let enum_name = format!("{}_{}", self.class_name.to_lowercase(), field.name.to_lowercase());
                                let values: Vec<String> = options.iter().map(|opt| opt.value.clone()).collect();
                                crate::db::create_or_update_enum(db, &enum_name, &values).await?;
                            }
                        },
                        // Query options don't need enum types
                        OptionsSource::Query { .. } => {}
                    }
                }
            }
        }
        
        // Register in entities registry if not exists
        let entity_exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM entity_registry WHERE class_name = $1)",
        )
        .bind(&self.class_name)
        .fetch_one(db)
        .await
        .map_err(Error::Database)?;
        
        // Insert or update the entity in the registry
        if entity_exists.0 {
            sqlx::query(
                "UPDATE entity_registry SET display_name = $2 WHERE class_name = $1",
            )
            .bind(&self.class_name)
            .bind(&self.display_name)
            .execute(db)
            .await
            .map_err(Error::Database)?;
        } else {
            sqlx::query(
                "INSERT INTO entity_registry (class_name, display_name) VALUES ($1, $2)",
            )
            .bind(&self.class_name)
            .bind(&self.display_name)
            .execute(db)
            .await
            .map_err(Error::Database)?;
        }
        
        Ok(())
    }

    /// Add a new enum with values
    pub async fn add_enum_with_values(&self, db: &PgPool, enum_name: &str, values: &[String]) -> Result<()> {
        // Create or update the enum type
        crate::db::create_or_update_enum(db, enum_name, values).await?;
        Ok(())
    }

    /// Add an enum field to the database with its values
    pub async fn add_enum_field(&self, db: &PgPool, _field_name: &str, enum_name: &str, values: &[String]) -> Result<()> {
        // Add enum type first
        create_or_update_enum(db, enum_name, values).await
    }
} 