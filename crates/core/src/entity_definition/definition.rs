use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, FromRow, Row};
use std::collections::HashMap;
use std::fmt::Debug;
use time::OffsetDateTime;
use uuid::Uuid;

use super::schema::Schema;
use crate::error::{Error, Result};
use crate::field::FieldDefinition;

/// Parameters for creating a new entity definition
#[derive(Debug, Clone)]
pub struct EntityDefinitionParams {
    /// Entity type identifier
    pub entity_type: String,
    /// Display name
    pub display_name: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional group name
    pub group_name: Option<String>,
    /// Whether entities of this type can have children
    pub allow_children: bool,
    /// Optional icon identifier
    pub icon: Option<String>,
    /// Field definitions
    pub fields: Vec<FieldDefinition>,
    /// UUID of the user creating this definition
    pub created_by: Uuid,
}

/// Function to serialize/deserialize `OffsetDateTime` with defaults
mod datetime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;

    pub fn serialize<S>(date: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = date.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        OffsetDateTime::parse(&s, &Rfc3339).map_or_else(|_| Ok(OffsetDateTime::now_utc()), Ok)
    }
}

/// An entity definition that describes the structure of an entity type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EntityDefinition {
    /// Unique identifier for this entity type definition
    #[serde(default)]
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
    pub fields: Vec<FieldDefinition>,
    /// Schema for this entity type
    pub schema: Schema,
    /// Created at timestamp
    #[serde(with = "datetime_serde")]
    pub created_at: OffsetDateTime,
    /// Updated at timestamp
    #[serde(with = "datetime_serde")]
    pub updated_at: OffsetDateTime,
    /// Created by user uuid
    pub created_by: Uuid,
    /// Updated by user uuid
    pub updated_by: Option<Uuid>,
    /// Whether this entity type is published
    pub published: bool,
    /// Version of this entity type
    #[serde(default = "default_version")]
    pub version: i32,
}

impl Default for EntityDefinition {
    fn default() -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            uuid: Uuid::nil(),
            entity_type: String::new(),
            display_name: String::new(),
            description: None,
            group_name: None,
            allow_children: false,
            icon: None,
            fields: Vec::new(),
            schema: Schema::default(),
            created_at: now,
            updated_at: now,
            created_by: Uuid::nil(),
            updated_by: None,
            published: false,
            version: default_version(),
        }
    }
}

/// Default version for new entities
const fn default_version() -> i32 {
    1
}

// Implement FromRow for EntityDefinition
impl<'r> FromRow<'r, PgRow> for EntityDefinition {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        let fields: Vec<FieldDefinition> =
            serde_json::from_value(row.try_get("field_definitions")?)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        // Create schema
        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            JsonValue::String(row.try_get::<String, _>("entity_type")?),
        );
        let schema = Schema::new(properties);

        Ok(Self {
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

impl EntityDefinition {
    /// Create a new entity type definition from parameters
    #[must_use]
    pub fn from_params(params: EntityDefinitionParams) -> Self {
        let entity_type = params.entity_type;
        let display_name = params.display_name;
        let description = params.description;
        let group_name = params.group_name;
        let allow_children = params.allow_children;
        let icon = params.icon;
        let fields = params.fields;
        let created_by = params.created_by;
        let now = OffsetDateTime::now_utc();

        // Create a properties map for the schema
        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            JsonValue::String(entity_type.clone()),
        );

        Self {
            uuid: Uuid::now_v7(),
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
            created_by,
            updated_by: None,
            published: false,
            version: 1,
        }
    }

    /// Create a new entity type definition (deprecated: use `from_params` instead)
    #[must_use]
    #[deprecated(note = "Use EntityDefinition::from_params instead")]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_type: String,
        display_name: String,
        description: Option<String>,
        group_name: Option<String>,
        allow_children: bool,
        icon: Option<String>,
        fields: Vec<FieldDefinition>,
        created_by: Uuid,
    ) -> Self {
        Self::from_params(EntityDefinitionParams {
            entity_type,
            display_name,
            description,
            group_name,
            allow_children,
            icon,
            fields,
            created_by,
        })
    }

    /// Get field definition by name
    #[must_use]
    pub fn get_field(&self, name: &str) -> Option<&FieldDefinition> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get all field definitions
    #[must_use]
    pub const fn get_fields(&self) -> &Vec<FieldDefinition> {
        &self.fields
    }

    /// Add field definition
    ///
    /// # Errors
    /// Returns `Error::FieldAlreadyExists` if a field with the same name already exists.
    pub fn add_field(&mut self, field_definition: FieldDefinition) -> Result<()> {
        if self.get_field(&field_definition.name).is_some() {
            return Err(Error::FieldAlreadyExists(field_definition.name));
        }
        self.fields.push(field_definition);
        Ok(())
    }

    /// Update field definition
    ///
    /// # Errors
    /// Returns `Error::FieldNotFound` if the field does not exist.
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
    ///
    /// # Errors
    /// Returns `Error::FieldNotFound` if the field does not exist.
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
    ///
    /// # Panics
    /// Panics if `Regex::new` fails, which should never happen with a valid regex pattern.
    ///
    /// # Errors
    /// Returns `Error::Validation` if the entity type name is invalid or required fields are missing.
    pub fn validate(&self) -> Result<()> {
        // Check for required fields
        if self.entity_type.is_empty() {
            return Err(Error::ValidationFailed(
                "Entity type cannot be empty".to_string(),
            ));
        }

        // Check that entity_type only contains alphanumeric characters and underscores
        let name_pattern = Regex::new(r"^[a-zA-Z0-9_]+$")
            .map_err(|e| Error::ValidationFailed(format!("Invalid regex pattern: {e}")))?;
        if !name_pattern.is_match(&self.entity_type) {
            return Err(Error::ValidationFailed(
                "Entity type must contain only alphanumeric characters and underscores (no spaces, hyphens, or special characters)".to_string(),
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
