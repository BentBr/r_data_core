use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, FromRow, Row};
use std::collections::HashMap;
use std::fmt::{Debug, Write};
use time::OffsetDateTime;
use uuid::Uuid;

use super::schema::Schema;
use crate::error::{Error, Result};
use crate::field::FieldDefinition;
use crate::field::FieldType;

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

    /// Get the SQL table name for this entity type
    #[must_use]
    pub fn get_table_name(&self) -> String {
        format!("entity_{}", self.entity_type.to_lowercase())
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
        let name_pattern = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
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
    #[must_use]
    pub fn generate_schema_sql(&self) -> String {
        let table_name = self.get_table_name();
        let mut sql = String::new();

        self.generate_create_table_sql(&mut sql, &table_name);
        self.generate_relation_tables_sql(&mut sql, &table_name);
        self.generate_indexes_sql(&mut sql, &table_name);

        sql
    }

    /// Generate CREATE TABLE statement with all columns
    #[allow(clippy::write_with_newline)]
    fn generate_create_table_sql(&self, sql: &mut String, table_name: &str) {
        let _ = write!(sql, "CREATE TABLE IF NOT EXISTS {table_name} (\n");
        sql.push_str("    uuid UUID PRIMARY KEY DEFAULT uuidv7(),\n");
        sql.push_str("    path TEXT,\n");
        sql.push_str("    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),\n");
        sql.push_str("    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),\n");
        sql.push_str("    created_by UUID,\n");
        sql.push_str("    updated_by UUID,\n");
        sql.push_str("    published BOOLEAN NOT NULL DEFAULT FALSE,\n");
        sql.push_str("    version INTEGER NOT NULL DEFAULT 1");

        for field in &self.fields {
            let field_name = &field.name;
            if matches!(field.field_type, FieldType::ManyToMany) {
                continue;
            }
            if matches!(field.field_type, FieldType::ManyToOne) {
                if field.validation.target_class.is_some() {
                    let _ = write!(sql, ",\n    {field_name}_uuid UUID");
                }
                continue;
            }

            let sql_type = crate::field::types::get_sql_type_for_field(
                &field.field_type,
                field.validation.max_length,
                field.validation.options_source.as_ref().and_then(|os| {
                    if let crate::field::OptionsSource::Enum { enum_name } = os {
                        Some(enum_name.as_str())
                    } else {
                        None
                    }
                }),
            );

            let _ = write!(sql, ",\n    {field_name} {sql_type}");
            if field.required {
                sql.push_str(" NOT NULL");
            }
        }
        sql.push_str("\n);\n\n");
    }

    /// Generate `ManyToMany` relation tables
    #[allow(clippy::write_with_newline)]
    fn generate_relation_tables_sql(&self, sql: &mut String, table_name: &str) {
        for field in &self.fields {
            if !matches!(field.field_type, FieldType::ManyToMany) {
                continue;
            }
            let Some(target_class) = &field.validation.target_class else {
                continue;
            };

            let relation_table = format!(
                "{}_{}_{}_relation",
                table_name,
                self.entity_type.to_lowercase(),
                target_class.to_lowercase()
            );
            let entity_lower = self.entity_type.to_lowercase();
            let target_lower = target_class.to_lowercase();

            let _ = write!(sql, "CREATE TABLE IF NOT EXISTS {relation_table} (\n");
            let _ = write!(
                sql,
                "    {entity_lower}_uuid UUID NOT NULL REFERENCES {table_name} (uuid),\n"
            );
            let _ = write!(sql, "    {target_lower}_uuid UUID NOT NULL,\n");
            sql.push_str("    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),\n");
            let _ = write!(
                sql,
                "    PRIMARY KEY ({entity_lower}_uuid, {target_lower}_uuid)\n);\n\n"
            );

            sql.push_str("-- INDEX: Relation table source index\n");
            let _ = write!(sql, "CREATE INDEX IF NOT EXISTS idx_{relation_table}_{entity_lower}_uuid ON {relation_table} ({entity_lower}_uuid);\n\n");
            sql.push_str("-- INDEX: Relation table target index\n");
            let _ = write!(sql, "CREATE INDEX IF NOT EXISTS idx_{relation_table}_{target_lower}_uuid ON {relation_table} ({target_lower}_uuid);\n\n");
        }
    }

    /// Generate index creation and drop statements
    #[allow(clippy::write_with_newline)]
    fn generate_indexes_sql(&self, sql: &mut String, table_name: &str) {
        // Create indexes for indexed fields
        for field in &self.fields {
            if !field.indexed {
                continue;
            }
            let field_name = &field.name;
            if matches!(field.field_type, FieldType::ManyToOne)
                && field.validation.target_class.is_some()
            {
                sql.push_str("-- INDEX: ManyToOne reference field index\n");
                let _ = write!(sql, "CREATE INDEX IF NOT EXISTS idx_{table_name}_{field_name}_uuid ON {table_name} ({field_name}_uuid);\n\n");
            } else if !matches!(field.field_type, FieldType::ManyToMany) {
                sql.push_str("-- INDEX: Regular field index\n");
                let _ = write!(sql, "CREATE INDEX IF NOT EXISTS idx_{table_name}_{field_name} ON {table_name} ({field_name});\n\n");
            }
        }

        // Handle unique constraints
        for field in &self.fields {
            let field_name = &field.name;
            if matches!(
                field.field_type,
                FieldType::ManyToMany | FieldType::ManyToOne
            ) {
                continue;
            }
            if field.unique {
                sql.push_str("-- UNIQUE: Field unique constraint\n");
                let _ = write!(sql, "CREATE UNIQUE INDEX IF NOT EXISTS idx_{table_name}_{field_name}_unique ON {table_name} ({field_name});\n\n");
            } else {
                sql.push_str("-- DROP UNIQUE: Remove unique constraint if exists\n");
                let _ = write!(
                    sql,
                    "DROP INDEX IF EXISTS idx_{table_name}_{field_name}_unique;\n\n"
                );
            }
        }

        // Drop indexes for non-indexed fields
        for field in &self.fields {
            if field.indexed || matches!(field.field_type, FieldType::ManyToMany) {
                continue;
            }
            let field_name = &field.name;
            if matches!(field.field_type, FieldType::ManyToOne)
                && field.validation.target_class.is_some()
            {
                sql.push_str("-- DROP INDEX: Remove index if exists\n");
                let _ = write!(
                    sql,
                    "DROP INDEX IF EXISTS idx_{table_name}_{field_name}_uuid;\n\n"
                );
            } else if !matches!(field.field_type, FieldType::ManyToOne) {
                sql.push_str("-- DROP INDEX: Remove index if exists\n");
                let _ = write!(
                    sql,
                    "DROP INDEX IF EXISTS idx_{table_name}_{field_name};\n\n"
                );
            }
        }
    }

    /// Returns the properly formatted table name for this entity definition
    #[must_use]
    pub fn table_name(&self) -> String {
        self.entity_type.to_lowercase()
    }

    /// Generate SQL schema for this entity definition
    /// This is an alias for `generate_schema_sql` to maintain compatibility
    #[must_use]
    pub fn generate_sql_schema(&self) -> String {
        self.generate_schema_sql()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::ui::UiSettings;
    use crate::field::FieldDefinition;

    fn create_test_entity_definition() -> EntityDefinition {
        EntityDefinition {
            uuid: Uuid::now_v7(),
            entity_type: "test".to_string(),
            display_name: "Test Entity".to_string(),
            description: None,
            group_name: None,
            allow_children: false,
            icon: None,
            fields: vec![FieldDefinition {
                name: "name".to_string(),
                display_name: "Name".to_string(),
                field_type: FieldType::String,
                description: None,
                required: false,
                indexed: false,
                filterable: false,
                unique: false,
                default_value: None,
                validation: crate::field::options::FieldValidation::default(),
                ui_settings: UiSettings::default(),
                constraints: std::collections::HashMap::new(),
            }],
            schema: Schema::default(),
            created_at: time::OffsetDateTime::now_utc(),
            updated_at: time::OffsetDateTime::now_utc(),
            created_by: Uuid::nil(),
            updated_by: None,
            published: false,
            version: 1,
        }
    }

    #[test]
    fn test_generate_schema_sql_includes_unique_index() {
        let mut def = create_test_entity_definition();
        def.fields[0].unique = true;

        let sql = def.generate_schema_sql();

        assert!(
            sql.contains("CREATE UNIQUE INDEX IF NOT EXISTS idx_entity_test_name_unique"),
            "SQL should contain unique index creation"
        );
        assert!(
            sql.contains("ON entity_test (name)"),
            "SQL should specify correct table and column"
        );
    }

    #[test]
    fn test_generate_schema_sql_no_unique_index_when_false() {
        let def = create_test_entity_definition();
        // unique is false by default

        let sql = def.generate_schema_sql();

        assert!(
            !sql.contains("CREATE UNIQUE INDEX"),
            "SQL should not CREATE unique index when unique is false"
        );
        assert!(
            sql.contains("DROP INDEX IF EXISTS"),
            "SQL should DROP unique index when unique is false (to clean up)"
        );
    }

    #[test]
    fn test_generate_schema_sql_no_unique_index_when_not_set() {
        let def = create_test_entity_definition();

        let sql = def.generate_schema_sql();

        assert!(
            !sql.contains("CREATE UNIQUE INDEX"),
            "SQL should not CREATE unique index when unique is not set"
        );
        assert!(
            sql.contains("DROP INDEX IF EXISTS"),
            "SQL should DROP unique index when unique is not set (to clean up)"
        );
    }

    #[test]
    fn test_generate_schema_sql_unique_index_comment() {
        let mut def = create_test_entity_definition();
        def.fields[0].unique = true;

        let sql = def.generate_schema_sql();

        assert!(
            sql.contains("-- UNIQUE: Field unique constraint"),
            "SQL should contain unique constraint comment"
        );
    }
}
