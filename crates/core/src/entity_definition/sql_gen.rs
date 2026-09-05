use std::fmt::Write;

use crate::field::FieldType;

use super::definition::EntityDefinition;

impl EntityDefinition {
    /// Get the SQL table name for this entity type
    #[must_use]
    pub fn get_table_name(&self) -> String {
        format!("entity_{}", self.entity_type.to_lowercase())
    }

    /// Returns the properly formatted table name for this entity definition
    #[must_use]
    pub fn table_name(&self) -> String {
        self.entity_type.to_lowercase()
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

    /// Generate SQL schema for this entity definition
    /// This is an alias for `generate_schema_sql` to maintain compatibility
    #[must_use]
    pub fn generate_sql_schema(&self) -> String {
        self.generate_schema_sql()
    }

    /// Generate CREATE TABLE statement with all columns
    fn generate_create_table_sql(&self, sql: &mut String, table_name: &str) {
        let _ = writeln!(sql, "CREATE TABLE IF NOT EXISTS {table_name} (");
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

            let _ = writeln!(sql, "CREATE TABLE IF NOT EXISTS {relation_table} (");
            let _ = writeln!(
                sql,
                "    {entity_lower}_uuid UUID NOT NULL REFERENCES {table_name} (uuid),"
            );
            let _ = writeln!(sql, "    {target_lower}_uuid UUID NOT NULL,");
            sql.push_str("    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),\n");
            let _ = writeln!(
                sql,
                "    PRIMARY KEY ({entity_lower}_uuid, {target_lower}_uuid)\n);\n"
            );

            sql.push_str("-- INDEX: Relation table source index\n");
            let _ = writeln!(sql, "CREATE INDEX IF NOT EXISTS idx_{relation_table}_{entity_lower}_uuid ON {relation_table} ({entity_lower}_uuid);\n");
            sql.push_str("-- INDEX: Relation table target index\n");
            let _ = writeln!(sql, "CREATE INDEX IF NOT EXISTS idx_{relation_table}_{target_lower}_uuid ON {relation_table} ({target_lower}_uuid);\n");
        }
    }

    /// Generate index creation and drop statements
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
                let _ = writeln!(sql, "CREATE INDEX IF NOT EXISTS idx_{table_name}_{field_name}_uuid ON {table_name} ({field_name}_uuid);\n");
            } else if !matches!(field.field_type, FieldType::ManyToMany) {
                sql.push_str("-- INDEX: Regular field index\n");
                let _ = writeln!(sql, "CREATE INDEX IF NOT EXISTS idx_{table_name}_{field_name} ON {table_name} ({field_name});\n");
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
                let _ = writeln!(sql, "CREATE UNIQUE INDEX IF NOT EXISTS idx_{table_name}_{field_name}_unique ON {table_name} ({field_name});\n");
            } else {
                sql.push_str("-- DROP UNIQUE: Remove unique constraint if exists\n");
                let _ = writeln!(
                    sql,
                    "DROP INDEX IF EXISTS idx_{table_name}_{field_name}_unique;\n"
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
                let _ = writeln!(
                    sql,
                    "DROP INDEX IF EXISTS idx_{table_name}_{field_name}_uuid;\n"
                );
            } else if !matches!(field.field_type, FieldType::ManyToOne) {
                sql.push_str("-- DROP INDEX: Remove index if exists\n");
                let _ = writeln!(sql, "DROP INDEX IF EXISTS idx_{table_name}_{field_name};\n");
            }
        }
    }
}
