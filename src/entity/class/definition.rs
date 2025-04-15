use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, FromRow, Row};
use std::collections::HashMap;
use std::fmt::Debug;
use uuid::timestamp;
use uuid::{ContextV7, Uuid};

use super::schema::Schema;
// Temporarily comment out missing function
// use crate::db::create_or_update_enum;
use crate::entity::field::FieldDefinition;
use crate::error::{Error, Result};

/// A class definition that describes the structure of an entity type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassDefinition {
    /// Unique identifier for this entity type definition
    pub uuid: Uuid,
    /// Entity type name, must be unique in the database
    pub entity_type: String,
    /// Display name for this entity type
    pub display_name: String,
    /// Description of this entity type
    pub description: Option<String>,
    /// Group name for organizing entity types
    pub group_name: Option<String>,
    /// Whether this entity type can have children
    pub allow_children: bool,
    /// Icon for this entity type
    pub icon: Option<String>,
    /// Field definitions for this entity type
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
    /// Schema for this entity type
    #[serde(default)]
    pub schema: Schema,
    /// Created at timestamp
    pub created_at: DateTime<Utc>,
    /// Updated at timestamp
    pub updated_at: DateTime<Utc>,
    /// Created by user uuid
    pub created_by: Option<Uuid>,
    /// Updated by user uuid
    pub updated_by: Option<Uuid>,
    /// Whether this entity type is published
    pub published: bool,
    /// Version of this entity type
    pub version: i32,
}

// Implement FromRow for ClassDefinition
impl<'r> FromRow<'r, PgRow> for ClassDefinition {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        let fields: Vec<FieldDefinition> = serde_json::from_value(row.try_get("fields")?)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        // Create schema
        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            JsonValue::String(row.try_get::<String, _>("entity_type")?),
        );
        let schema = Schema::new(properties);

        Ok(ClassDefinition {
            uuid: row.try_get("uuid")?,
            entity_type: row.try_get("entity_type")?,
            display_name: row.try_get("display_name")?,
            description: row.try_get("description")?,
            group_name: row.try_get("group_name")?,
            allow_children: row.try_get("allow_children")?,
            icon: row.try_get("icon")?,
            fields,
            schema,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            created_by: row.try_get("created_by")?,
            updated_by: row.try_get("updated_by")?,
            published: row.try_get("published")?,
            version: row.try_get("version")?,
        })
    }
}

impl ClassDefinition {
    /// Create a new entity type definition
    pub fn new(
        entity_type: String,
        display_name: String,
        description: Option<String>,
        group_name: Option<String>,
        allow_children: bool,
        icon: Option<String>,
        fields: Vec<FieldDefinition>,
    ) -> Self {
        let now = Utc::now();
        let context = ContextV7::new();
        let ts = timestamp::Timestamp::now(&context);

        // Create a properties map for the schema
        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            JsonValue::String(entity_type.clone()),
        );

        ClassDefinition {
            uuid: Uuid::new_v7(ts),
            entity_type,
            display_name,
            description,
            group_name,
            allow_children,
            icon,
            fields,
            schema: Schema::new(properties),
            created_at: now,
            updated_at: now,
            created_by: None,
            updated_by: None,
            published: false,
            version: 1,
        }
    }

    /// Get the SQL table name for this entity type
    pub fn get_table_name(&self) -> String {
        format!("{}_entities", self.entity_type.to_lowercase())
    }

    /// Get field definition by name
    pub fn get_field(&self, name: &str) -> Option<&FieldDefinition> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get all field definitions
    pub fn get_fields(&self) -> &Vec<FieldDefinition> {
        &self.fields
    }

    /// Add field definition
    pub fn add_field(&mut self, field_definition: FieldDefinition) -> Result<()> {
        if self.get_field(&field_definition.name).is_some() {
            return Err(Error::FieldAlreadyExists(field_definition.name));
        }
        self.fields.push(field_definition);
        Ok(())
    }

    /// Update field definition
    pub fn update_field(&mut self, field_definition: FieldDefinition) -> Result<()> {
        let field_idx = self
            .fields
            .iter()
            .position(|f| f.name == field_definition.name);

        match field_idx {
            Some(idx) => {
                self.fields[idx] = field_definition;
                Ok(())
            }
            None => Err(Error::FieldNotFound(field_definition.name)),
        }
    }

    /// Remove field definition
    pub fn remove_field(&mut self, name: &str) -> Result<()> {
        let field_idx = self.fields.iter().position(|f| f.name == name);

        match field_idx {
            Some(idx) => {
                self.fields.remove(idx);
                Ok(())
            }
            None => Err(Error::FieldNotFound(name.to_string())),
        }
    }

    /// Validate the entity type definition
    pub fn validate(&self) -> Result<()> {
        // Check for required fields
        if self.entity_type.is_empty() {
            return Err(Error::ValidationFailed(
                "Entity type cannot be empty".to_string(),
            ));
        }

        if self.display_name.is_empty() {
            return Err(Error::ValidationFailed(
                "Display name cannot be empty".to_string(),
            ));
        }

        // Check for duplicate field names
        let mut field_names = std::collections::HashSet::new();
        for field in &self.fields {
            if !field_names.insert(&field.name) {
                return Err(Error::ValidationFailed(format!(
                    "Duplicate field name: {}",
                    field.name
                )));
            }

            // Validate each field
            field.validate()?;
        }

        Ok(())
    }
}
