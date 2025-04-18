use chrono::{DateTime, Utc};
use regex;
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
use crate::entity::field::FieldType;
use crate::error::{Error, Result};

/// A class definition that describes the structure of an entity type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassDefinition {
    /// Unique identifier for this entity type definition
    #[serde(default = "generate_uuid")]
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
    #[serde(default = "default_datetime")]
    pub created_at: DateTime<Utc>,
    /// Updated at timestamp
    #[serde(default = "default_datetime")]
    pub updated_at: DateTime<Utc>,
    /// Created by user uuid
    #[serde(default)]
    pub created_by: Option<Uuid>,
    /// Updated by user uuid
    #[serde(default)]
    pub updated_by: Option<Uuid>,
    /// Whether this entity type is published
    #[serde(default)]
    pub published: bool,
    /// Version of this entity type
    #[serde(default = "default_version")]
    pub version: i32,
}

/// Generate a new UUID v7 for deserialization defaults
fn generate_uuid() -> Uuid {
    let context = uuid::ContextV7::new();
    let ts = uuid::timestamp::Timestamp::now(&context);
    Uuid::new_v7(ts)
}

/// Default datetime for created_at and updated_at
fn default_datetime() -> DateTime<Utc> {
    Utc::now()
}

/// Default version for new entities
fn default_version() -> i32 {
    1
}

// Implement FromRow for ClassDefinition
impl<'r> FromRow<'r, PgRow> for ClassDefinition {
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
        format!("entity_{}", self.entity_type.to_lowercase())
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

        // Check that entity_type only contains alphanumeric characters and underscores
        let name_pattern = regex::Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
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

    /// Generate SQL table schema for this class
    pub fn generate_schema_sql(&self) -> String {
        let table_name = self.get_table_name();
        let mut sql = String::new();

        // Check if table exists and create it if not
        sql.push_str(&format!("CREATE TABLE IF NOT EXISTS {} (\n", table_name));
        sql.push_str("    uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),\n");
        sql.push_str("    path TEXT NOT NULL,\n");
        sql.push_str("    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),\n");
        sql.push_str("    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),\n");
        sql.push_str("    created_by UUID,\n");
        sql.push_str("    updated_by UUID,\n");
        sql.push_str("    published BOOLEAN NOT NULL DEFAULT FALSE,\n");
        sql.push_str("    version INTEGER NOT NULL DEFAULT 1");

        // Add all field columns directly to the CREATE TABLE statement
        for field in &self.fields {
            let field_name = field.name.clone();

            // Skip relation fields as they'll be handled separately
            if matches!(field.field_type, FieldType::ManyToMany) {
                continue;
            }

            // For ManyToOne, add a reference column
            if matches!(field.field_type, FieldType::ManyToOne) {
                if let Some(target_class) = &field.validation.target_class {
                    sql.push_str(&format!(",\n    {}_uuid UUID", field_name));
                }
                continue;
            }

            // Use the field's get_sql_type method to determine the SQL type
            let sql_type = crate::entity::field::types::get_sql_type_for_field(
                &field.field_type,
                field.validation.max_length,
                field.validation.options_source.as_ref().and_then(|os| {
                    if let crate::entity::field::OptionsSource::Enum { enum_name } = os {
                        Some(enum_name.as_str())
                    } else {
                        None
                    }
                }),
            );

            // Add NOT NULL constraint if required
            sql.push_str(&format!(",\n    {} {}", field_name, sql_type));
            if field.required {
                sql.push_str(" NOT NULL");
            }
        }

        sql.push_str("\n);\n\n");

        // Add indexes for common search fields
        sql.push_str(&format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_path ON {} (path);\n\n",
            table_name, table_name
        ));

        // Add indexes for fields marked as indexed
        for field in &self.fields {
            if field.indexed {
                let field_name = &field.name;

                // For ManyToOne relations, index the reference column
                if matches!(field.field_type, FieldType::ManyToOne) {
                    if let Some(_) = &field.validation.target_class {
                        sql.push_str(&format!(
                            "CREATE INDEX IF NOT EXISTS idx_{}_{}_uuid ON {} ({}_uuid);\n\n",
                            table_name, field_name, table_name, field_name
                        ));
                    }
                } else if !matches!(field.field_type, FieldType::ManyToMany) {
                    // Don't add index for ManyToMany as those are in separate tables
                    sql.push_str(&format!(
                        "CREATE INDEX IF NOT EXISTS idx_{}_{} ON {} ({});\n\n",
                        table_name, field_name, table_name, field_name
                    ));
                }
            }
        }

        // Create relationship tables for ManyToMany relations
        for field in &self.fields {
            if matches!(field.field_type, FieldType::ManyToMany) {
                if let Some(target_class) = &field.validation.target_class {
                    let relation_table = format!(
                        "{}_{}_{}_relation",
                        table_name,
                        self.entity_type.to_lowercase(),
                        target_class.to_lowercase()
                    );

                    sql.push_str(&format!(
                        "CREATE TABLE IF NOT EXISTS {} (\n",
                        relation_table
                    ));
                    sql.push_str(&format!(
                        "    {}_uuid UUID NOT NULL REFERENCES {} (uuid),\n",
                        self.entity_type.to_lowercase(),
                        table_name
                    ));
                    sql.push_str(&format!(
                        "    {}_uuid UUID NOT NULL,\n",
                        target_class.to_lowercase()
                    ));
                    sql.push_str(
                        "    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),\n",
                    );
                    sql.push_str(&format!(
                        "    PRIMARY KEY ({}_uuid, {}_uuid)\n",
                        self.entity_type.to_lowercase(),
                        target_class.to_lowercase()
                    ));
                    sql.push_str(");\n\n");

                    // Add indexes for the relation table
                    sql.push_str(&format!(
                        "CREATE INDEX IF NOT EXISTS idx_{}_source ON {} ({}_uuid);\n",
                        relation_table,
                        relation_table,
                        self.entity_type.to_lowercase()
                    ));
                    sql.push_str(&format!(
                        "CREATE INDEX IF NOT EXISTS idx_{}_target ON {} ({}_uuid);\n\n",
                        relation_table,
                        relation_table,
                        target_class.to_lowercase()
                    ));
                }
            }
        }

        // Add the auto-update timestamp trigger function if it doesn't exist
        sql.push_str("-- Create auto-update timestamp function if it doesn't exist\n");
        sql.push_str("DO $$ BEGIN\n");
        sql.push_str(
            "    IF NOT EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'update_timestamp') THEN\n",
        );
        sql.push_str("        CREATE FUNCTION update_timestamp() RETURNS TRIGGER AS $$\n");
        sql.push_str("        BEGIN\n");
        sql.push_str("            NEW.updated_at = NOW();\n");
        sql.push_str("            RETURN NEW;\n");
        sql.push_str("        END;\n");
        sql.push_str("        $$ LANGUAGE plpgsql;\n");
        sql.push_str("    END IF;\n");
        sql.push_str("END $$;\n\n");

        // Create a unique trigger for this table
        sql.push_str(&format!(
            "DROP TRIGGER IF EXISTS set_timestamp_{0} ON {0};\n",
            table_name
        ));
        sql.push_str(&format!("CREATE TRIGGER set_timestamp_{0}\n", table_name));
        sql.push_str(&format!("BEFORE UPDATE ON {}\n", table_name));
        sql.push_str("FOR EACH ROW\n");
        sql.push_str("EXECUTE FUNCTION update_timestamp();\n");

        sql
    }

    /// Returns the properly formatted table name for this class definition
    pub fn table_name(&self) -> String {
        self.entity_type.to_lowercase()
    }

    /// Generate SQL schema for this class definition
    /// This is an alias for generate_schema_sql to maintain compatibility
    pub fn generate_sql_schema(&self) -> String {
        self.generate_schema_sql()
    }

    /// Convert to API schema model
    pub fn to_schema_model(
        &self,
    ) -> crate::api::admin::class_definitions::models::ClassDefinitionSchema {
        use crate::api::admin::class_definitions::models::ClassDefinitionSchema;

        ClassDefinitionSchema {
            uuid: Some(self.uuid),
            entity_type: self.entity_type.clone(),
            display_name: self.display_name.clone(),
            description: self.description.clone(),
            group_name: self.group_name.clone(),
            allow_children: self.allow_children,
            icon: self.icon.clone(),
            fields: self.fields.iter().map(|f| f.to_schema_model()).collect(),
            published: Some(self.published),
        }
    }
}
