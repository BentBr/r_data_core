use super::definition::ClassDefinition;
use crate::entity::field::{get_sql_type_for_field, FieldType, OptionsSource};
use crate::error::Error as AppError;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::postgres::PgPool;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub properties: HashMap<String, JsonValue>,
}

impl Schema {
    pub fn new(properties: HashMap<String, JsonValue>) -> Self {
        Self { properties }
    }

    pub fn get_fields(&self) -> impl Iterator<Item = (&String, &JsonValue)> {
        self.properties.iter()
    }

    pub fn get_column_definitions(&self) -> Result<Vec<String>, Error> {
        let mut columns = Vec::new();

        if let Some(properties) = self.properties.get("properties") {
            if let Some(props) = properties.as_object() {
                for (field_name, field_def) in props {
                    if let Some(field_type) = field_def.get("type") {
                        let sql_type = match field_type.as_str() {
                            Some("string") => {
                                if let Some(max_length) = field_def.get("maxLength") {
                                    format!("VARCHAR({})", max_length)
                                } else {
                                    "TEXT".to_string()
                                }
                            }
                            Some("number") => "NUMERIC".to_string(),
                            Some("integer") => "BIGINT".to_string(),
                            Some("boolean") => "BOOLEAN".to_string(),
                            Some("array") | Some("object") => "JSONB".to_string(),
                            Some("date") => "DATE".to_string(),
                            Some("datetime") => "TIMESTAMP WITH TIME ZONE".to_string(),
                            Some("uuid") => "UUID".to_string(),
                            _ => "TEXT".to_string(),
                        };

                        let mut column_def = format!("{} {}", field_name, sql_type);

                        if let Some(required) = field_def.get("required") {
                            if required.as_bool() == Some(true) {
                                column_def.push_str(" NOT NULL");
                            }
                        }

                        columns.push(column_def);
                    }
                }
            }
        }

        Ok(columns)
    }

    pub fn get_table_name(&self) -> String {
        if let Some(entity_type) = self.properties.get("entity_type") {
            if let Some(entity_type_str) = entity_type.as_str() {
                return format!("{}_entities", entity_type_str);
            }
        }
        "unknown_entities".to_string()
    }

    pub fn generate_sql_schema(&self) -> String {
        let table_name = self.get_table_name();
        let columns = self.get_column_definitions().unwrap_or_default();

        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table_name);
        sql.push_str("    uuid UUID PRIMARY KEY,\n");

        for (i, column) in columns.iter().enumerate() {
            sql.push_str("    ");
            sql.push_str(column);
            if i < columns.len() - 1 {
                sql.push_str(",\n");
            }
        }

        sql.push_str("\n);");
        sql
    }

    pub async fn apply_to_database(&self, pool: &PgPool) -> Result<(), AppError> {
        let sql = self.get_create_table_sql()?;
        sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e))?;
        Ok(())
    }
}

impl ClassDefinition {
    /// Generate SQL table schema for this class
    pub fn generate_sql_schema(&self) -> String {
        let mut columns = vec![
            "id UUID PRIMARY KEY DEFAULT uuid_generate_v7()",
            "uuid UUID NOT NULL UNIQUE",
            "path TEXT NOT NULL",
            "created_at TIMESTAMP WITH TIME ZONE NOT NULL",
            "updated_at TIMESTAMP WITH TIME ZONE NOT NULL",
            "created_by UUID",
            "updated_by UUID",
            "published BOOLEAN NOT NULL DEFAULT FALSE",
            "version INTEGER NOT NULL DEFAULT 1",
        ];

        // Add custom fields
        if let Some(props) = self.schema.get_fields() {
            if let JsonValue::Object(_) = props {
                let mut columns = Vec::<String>::new();
                for (field_name, field_obj) in self.get_fields().iter().filter(|f| {
                    !matches!(f.field_type, FieldType::ManyToOne | FieldType::ManyToMany)
                }) {
                    let sql_type = get_sql_type_for_field(
                        &field_obj.field_type,
                        field_obj.validation.max_length,
                        if let Some(OptionsSource::Enum { enum_name }) =
                            &field_obj.validation.options_source
                        {
                            Some(enum_name)
                        } else {
                            None
                        },
                    );
                    let null_constraint = if field_obj.required { " NOT NULL" } else { "" };
                    columns.push(format!("{} {}{}", field_name, sql_type, null_constraint));
                }
            }
        }

        // Add custom_fields JSONB for any additional fields not in schema
        columns.push("custom_fields JSONB NOT NULL DEFAULT '{}'");

        // Create table SQL
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", self.get_schema_table_name());
        sql.push_str(&columns.join(",\n"));
        sql.push_str("\n);\n");

        // Add indexes for searchable fields
        sql.push_str(&format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_uuid ON {} (uuid);\n",
            self.get_schema_table_name(),
            self.get_schema_table_name()
        ));

        // Generate relation tables
        for field in self
            .get_fields()
            .iter()
            .filter(|f| matches!(f.field_type, FieldType::ManyToOne | FieldType::ManyToMany))
        {
            if let Some(target_class) = &field.validation.target_class {
                let target_table = format!("entity_{}", target_class.to_lowercase());

                // For ManyToOne, add foreign key constraint
                if matches!(field.field_type, FieldType::ManyToOne) {
                    sql.push_str(&format!(
                        "ALTER TABLE {} ADD CONSTRAINT fk_{}_{} FOREIGN KEY ({}_uuid) REFERENCES {} (uuid) ON DELETE SET NULL;\n",
                        self.get_schema_table_name(),
                        self.get_schema_table_name(),
                        field.name,
                        field.name,
                        target_table
                    ));
                }

                // For ManyToMany, create a join table
                if matches!(field.field_type, FieldType::ManyToMany) {
                    let relation_table = format!(
                        "{}_{}_{}_relation",
                        self.entity_type.to_lowercase(),
                        field.name,
                        target_class.to_lowercase()
                    );

                    sql.push_str(&format!(
                        "CREATE TABLE IF NOT EXISTS {} (\n",
                        relation_table
                    ));
                    sql.push_str("  uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),\n");

                    // Reference to this entity
                    sql.push_str(&format!(
                        "  {}_uuid UUID NOT NULL REFERENCES {} (uuid) ON DELETE CASCADE,\n",
                        self.entity_type.to_lowercase(),
                        self.get_schema_table_name()
                    ));

                    // Reference to target entity
                    sql.push_str(&format!(
                        "  {}_uuid UUID NOT NULL REFERENCES {} (uuid) ON DELETE CASCADE,\n",
                        target_class.to_lowercase(),
                        target_table
                    ));

                    // Add position field for ordered relations and metadata
                    sql.push_str("  position INTEGER NOT NULL DEFAULT 0,\n");
                    sql.push_str("  metadata JSONB,\n");

                    // Add unique constraint to prevent duplicates
                    sql.push_str(&format!(
                        "  UNIQUE({}_uuid, {}_uuid)\n",
                        self.entity_type.to_lowercase(),
                        target_class.to_lowercase()
                    ));
                    sql.push_str(");\n");

                    // Add indices for faster lookups
                    sql.push_str(&format!(
                        "CREATE INDEX IF NOT EXISTS idx_{}_source ON {} ({}_uuid);\n",
                        relation_table,
                        relation_table,
                        self.entity_type.to_lowercase()
                    ));

                    sql.push_str(&format!(
                        "CREATE INDEX IF NOT EXISTS idx_{}_target ON {} ({}_uuid);\n",
                        relation_table,
                        relation_table,
                        target_class.to_lowercase()
                    ));
                }
            }
        }

        sql
    }

    /// Generate relation tables for this class
    fn generate_relation_tables(&self) -> String {
        let mut sql = String::new();
        let table_name = self.get_schema_table_name();

        for field in &self.schema.get_fields() {
            if matches!(
                field.field_type,
                FieldType::ManyToOne | FieldType::ManyToMany
            ) {
                if let Some(target_class) = &field.validation.target_class {
                    let target_table = format!("entity_{}", target_class.to_lowercase());

                    // For ManyToOne, add foreign key constraint
                    if matches!(field.field_type, FieldType::ManyToOne) {
                        sql.push_str(&format!(
                            "ALTER TABLE {} ADD CONSTRAINT fk_{}_{} FOREIGN KEY ({}_uuid) REFERENCES {} (uuid) ON DELETE SET NULL;\n",
                            table_name, table_name, field.name, field.name, target_table
                        ));
                    }

                    // For ManyToMany, create a join table
                    if matches!(field.field_type, FieldType::ManyToMany) {
                        let relation_table = format!(
                            "{}_{}_{}_relation",
                            self.entity_type.to_lowercase(),
                            field.name,
                            target_class.to_lowercase()
                        );

                        sql.push_str(&format!(
                            "CREATE TABLE IF NOT EXISTS {} (\n",
                            relation_table
                        ));
                        sql.push_str("  uuid BIGSERIAL PRIMARY KEY,\n");

                        // Reference to this entity
                        sql.push_str(&format!(
                            "  {}_uuid BIGINT NOT NULL REFERENCES {} (uuid) ON DELETE CASCADE,\n",
                            self.entity_type.to_lowercase(),
                            table_name
                        ));

                        // Reference to target entity
                        sql.push_str(&format!(
                            "  {}_uuid BIGINT NOT NULL REFERENCES {} (uuid) ON DELETE CASCADE,\n",
                            target_class.to_lowercase(),
                            target_table
                        ));

                        // Add position field for ordered relations and metadata
                        sql.push_str("  position INTEGER NOT NULL DEFAULT 0,\n");
                        sql.push_str("  metadata JSONB,\n");

                        // Add unique constraint to prevent duplicates
                        sql.push_str(&format!(
                            "  UNIQUE({}_uuid, {}_uuid)\n",
                            self.entity_type.to_lowercase(),
                            target_class.to_lowercase()
                        ));
                        sql.push_str(");\n");

                        // Add indices for faster lookups
                        sql.push_str(&format!(
                            "CREATE INDEX IF NOT EXISTS idx_{}_source ON {} ({}_uuid);\n",
                            relation_table,
                            relation_table,
                            self.entity_type.to_lowercase()
                        ));

                        sql.push_str(&format!(
                            "CREATE INDEX IF NOT EXISTS idx_{}_target ON {} ({}_uuid);\n",
                            relation_table,
                            relation_table,
                            target_class.to_lowercase()
                        ));
                    }
                }
            }
        }

        sql
    }

    pub async fn apply_to_database(&self, pool: &PgPool) -> Result<(), Error> {
        let sql = self.get_create_table_sql()?;
        sqlx::query(&sql).execute(pool).await?;
        Ok(())
    }

    pub fn get_drop_table_sql(&self) -> String {
        format!("DROP TABLE IF EXISTS {}", self.get_schema_table_name())
    }

    pub fn get_create_table_sql(&self) -> Result<String, Error> {
        let table_name = self.get_schema_table_name();
        let columns = self.get_column_definitions()?;
        Ok(format!(
            "CREATE TABLE IF NOT EXISTS {} ({})",
            table_name,
            columns.join(", ")
        ))
    }

    pub fn get_schema_table_name(&self) -> String {
        format!("entity_{}", self.entity_type.to_lowercase())
    }
}

impl From<JsonValue> for Schema {
    fn from(value: JsonValue) -> Self {
        if let JsonValue::Object(map) = value {
            Self {
                properties: map.into_iter().collect(),
            }
        } else {
            Self {
                properties: HashMap::new(),
            }
        }
    }
}

impl From<Schema> for JsonValue {
    fn from(schema: Schema) -> Self {
        JsonValue::Object(schema.properties.into_iter().collect())
    }
}
