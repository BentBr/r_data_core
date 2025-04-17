use crate::db::PgPoolExtension;
use crate::entity::ClassDefinition;
use crate::error::{Error, Result};
use log;
use regex;
use serde_json;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ClassDefinitionRepository {
    db_pool: PgPool,
}

impl ClassDefinitionRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<ClassDefinition>> {
        self.db_pool
            .repository_with_table::<ClassDefinition>("class_definitions")
            .list(None, Some("entity_type ASC"), Some(limit), Some(offset))
            .await
    }

    pub async fn get_by_uuid(&self, uuid: &Uuid) -> Result<ClassDefinition> {
        self.db_pool
            .repository_with_table::<ClassDefinition>("class_definitions")
            .get_by_uuid(uuid)
            .await
    }

    pub async fn create(&self, definition: &ClassDefinition) -> Result<Uuid> {
        // We need a custom implementation because the general repository requires a path field
        // that class definitions don't have

        // Convert the definition to JSON
        let json = serde_json::to_value(definition).map_err(|e| Error::Serialization(e))?;

        // Build the SQL fields and values
        let uuid = definition.uuid;
        let entity_type = &definition.entity_type;
        let display_name = &definition.display_name;
        let description = definition.description.as_ref();
        let group_name = definition.group_name.as_ref();
        let allow_children = definition.allow_children;
        let icon = definition.icon.as_ref();
        let fields =
            serde_json::to_value(&definition.fields).map_err(|e| Error::Serialization(e))?;
        let created_at = definition.created_at;
        let updated_at = definition.updated_at;
        let created_by = definition.created_by;
        let updated_by = definition.updated_by;
        let published = definition.published;
        let version = definition.version;

        // SQL query
        let query = "INSERT INTO class_definitions 
                    (uuid, entity_type, display_name, description, group_name, allow_children, 
                     icon, field_definitions, created_at, updated_at, created_by, updated_by, 
                     published, version) 
                    VALUES 
                    ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) 
                    RETURNING uuid";

        let result = sqlx::query_scalar::<_, Uuid>(query)
            .bind(uuid)
            .bind(entity_type)
            .bind(display_name)
            .bind(description)
            .bind(group_name)
            .bind(allow_children)
            .bind(icon)
            .bind(fields)
            .bind(created_at)
            .bind(updated_at)
            .bind(created_by)
            .bind(updated_by)
            .bind(published)
            .bind(version)
            .fetch_one(&self.db_pool)
            .await
            .map_err(Error::Database)?;

        Ok(result)
    }

    pub async fn update(&self, uuid: &Uuid, definition: &ClassDefinition) -> Result<()> {
        // Custom implementation for class definitions that doesn't require a path field

        // Build the SQL fields and values
        let entity_type = &definition.entity_type;
        let display_name = &definition.display_name;
        let description = definition.description.as_ref();
        let group_name = definition.group_name.as_ref();
        let allow_children = definition.allow_children;
        let icon = definition.icon.as_ref();
        let fields =
            serde_json::to_value(&definition.fields).map_err(|e| Error::Serialization(e))?;
        let updated_at = definition.updated_at;
        let updated_by = definition.updated_by;
        let published = definition.published;
        let version = definition.version;

        // SQL query
        let query = "UPDATE class_definitions SET 
                    entity_type = $1, 
                    display_name = $2, 
                    description = $3, 
                    group_name = $4, 
                    allow_children = $5, 
                    icon = $6, 
                    field_definitions = $7, 
                    updated_at = $8, 
                    updated_by = $9, 
                    published = $10, 
                    version = $11 
                    WHERE uuid = $12";

        sqlx::query(query)
            .bind(entity_type)
            .bind(display_name)
            .bind(description)
            .bind(group_name)
            .bind(allow_children)
            .bind(icon)
            .bind(fields)
            .bind(updated_at)
            .bind(updated_by)
            .bind(published)
            .bind(version)
            .bind(uuid)
            .execute(&self.db_pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    pub async fn delete(&self, uuid: &Uuid) -> Result<()> {
        self.db_pool
            .repository_with_table::<ClassDefinition>("class_definitions")
            .delete(uuid)
            .await
    }

    pub async fn apply_schema(&self, schema_sql: &str) -> Result<()> {
        // Split the schema into individual statements
        let mut statements = Vec::new();
        let mut current_statement = String::new();
        let mut in_do_block = false;
        let mut do_block_level = 0;

        for line in schema_sql.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Skip comments
            if trimmed.starts_with("--") {
                continue;
            }

            // Check if we're entering a DO block
            if trimmed.starts_with("DO") {
                in_do_block = true;
            }

            // Track BEGIN/END pairs in DO blocks
            if in_do_block {
                if trimmed.contains("BEGIN") {
                    do_block_level += 1;
                }
                if trimmed.contains("END") {
                    do_block_level -= 1;
                    if do_block_level == 0 && trimmed.contains("$$") {
                        in_do_block = false;
                    }
                }
            }

            current_statement.push_str(line);
            current_statement.push('\n');

            // If the line ends with a semicolon and we're not in a DO block,
            // or if we've reached the end of a DO block, add the statement
            if (trimmed.ends_with(";") && !in_do_block)
                || (trimmed.contains("END $$;") && !in_do_block)
            {
                statements.push(current_statement.trim().to_string());
                current_statement.clear();
            }
        }

        // Log information about the statements to be executed
        log::debug!(
            "Executing {} SQL statements for schema application",
            statements.len()
        );

        // Execute each statement
        for (i, statement) in statements.iter().enumerate() {
            if !statement.is_empty() {
                log::debug!("Executing statement #{}: {}", i + 1, statement);
                match sqlx::query(statement).execute(&self.db_pool).await {
                    Ok(_) => log::debug!("Statement #{} executed successfully", i + 1),
                    Err(e) => {
                        log::error!("Error executing statement #{}: {}", i + 1, e);
                        return Err(Error::Database(e));
                    }
                }
            }
        }

        Ok(())
    }

    /// Update entity table when fields change in a class definition
    pub async fn update_entity_table_for_class_definition(
        &self,
        definition: &ClassDefinition,
    ) -> Result<()> {
        // Get table name
        let table_name = definition.get_table_name();

        // Check if table exists
        let table_exists = self.check_table_exists(&table_name).await?;

        if !table_exists {
            // If table doesn't exist, apply the full schema
            let schema_sql = definition.generate_schema_sql();
            self.apply_schema(&schema_sql).await?;
            return Ok(());
        }

        // Table exists, check for fields that need to be added
        let current_columns = self.get_table_columns(&table_name).await?;
        log::info!("Current columns for {}: {:?}", table_name, current_columns);

        // Generate SQL for missing columns
        let mut alter_sql = String::new();

        for field in definition.fields.iter() {
            let field_name = &field.name;

            // Skip if column already exists
            if current_columns.contains(&field_name.to_lowercase()) {
                continue;
            }

            // Generate SQL type for this field
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
            let mut column_def = format!("{} {}", field_name, sql_type);
            if field.required {
                column_def.push_str(" NOT NULL");
            }

            // Add column
            alter_sql.push_str(&format!(
                "ALTER TABLE {} ADD COLUMN {} {};\n",
                table_name, field_name, sql_type
            ));

            // Add index if needed
            if field.indexed {
                alter_sql.push_str(&format!(
                    "CREATE INDEX IF NOT EXISTS idx_{}_{} ON {} ({});\n",
                    table_name, field_name, table_name, field_name
                ));
            }
        }

        // Apply the ALTER statements if any
        if !alter_sql.is_empty() {
            log::info!(
                "Applying ALTER statements for {}: {}",
                table_name,
                alter_sql
            );
            self.apply_schema(&alter_sql).await?;
        }

        Ok(())
    }

    /// Get column names for a table
    pub async fn get_table_columns(&self, table_name: &str) -> Result<Vec<String>> {
        let query = "
            SELECT column_name 
            FROM information_schema.columns 
            WHERE table_schema = 'public' AND table_name = $1
            ORDER BY column_name
        ";

        let columns: Vec<(String,)> = sqlx::query_as(query)
            .bind(table_name.to_lowercase())
            .fetch_all(&self.db_pool)
            .await
            .map_err(Error::Database)?;

        Ok(columns.into_iter().map(|c| c.0.to_lowercase()).collect())
    }

    pub async fn check_table_exists(&self, table_name: &str) -> Result<bool> {
        let result: (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = $1
            )",
        )
        .bind(table_name.to_lowercase())
        .fetch_one(&self.db_pool)
        .await?;

        Ok(result.0)
    }

    pub async fn count_table_records(&self, table_name: &str) -> Result<i64> {
        let result: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table_name))
            .fetch_one(&self.db_pool)
            .await?;

        Ok(result.0)
    }

    pub async fn delete_from_entity_registry(&self, entity_type: &str) -> Result<()> {
        sqlx::query("DELETE FROM entities WHERE name = $1")
            .bind(entity_type)
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }

    pub async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<ClassDefinition>> {
        let sql = "SELECT * FROM class_definitions WHERE entity_type = $1";
        let result = sqlx::query_as::<_, ClassDefinition>(sql)
            .bind(entity_type)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(result)
    }
}
