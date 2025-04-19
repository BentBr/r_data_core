use crate::db::PgPoolExtension;
use crate::entity::class::definition::ClassDefinition;
use crate::entity::field::types::FieldType;
use crate::error::{Error, Result};
use log;
use serde_json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::collections::HashSet;
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
        // Use custom query with explicit type casting
        let class_def = sqlx::query!(
            r#"
            SELECT 
                uuid, entity_type, display_name, description, group_name, 
                allow_children, icon, field_definitions as "field_definitions: serde_json::Value",
                created_at, updated_at, 
                created_by as "created_by: Uuid", updated_by,
                published, version
            FROM class_definitions
            WHERE uuid = $1
            "#,
            uuid
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(Error::Database)?
        .ok_or_else(|| Error::NotFound(format!("Class definition with UUID {} not found", uuid)))?;

        // Create schema
        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(class_def.entity_type.clone()),
        );
        let schema = crate::entity::class::schema::Schema::new(properties);

        // Convert to ClassDefinition
        Ok(ClassDefinition {
            uuid: class_def.uuid,
            entity_type: class_def.entity_type,
            display_name: class_def.display_name,
            description: class_def.description,
            group_name: class_def.group_name,
            allow_children: class_def.allow_children,
            icon: class_def.icon,
            fields: serde_json::from_value(class_def.field_definitions)
                .map_err(|e| Error::Serialization(e))?,
            schema,
            created_at: class_def.created_at,
            updated_at: class_def.updated_at,
            created_by: class_def.created_by,
            updated_by: class_def.updated_by,
            published: class_def.published,
            version: class_def.version,
        })
    }

    pub async fn create(&self, definition: &ClassDefinition) -> Result<Uuid> {
        // We need a custom implementation because the general repository requires a path field
        // that class definitions don't have

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
        // First, get the class definition to get the entity type
        let class_definition = self.get_by_uuid(uuid).await?;
        let table_name = class_definition.get_table_name();

        // Drop the entity table if it exists
        let table_exists = self.check_table_exists(&table_name).await?;
        if table_exists {
            // Also drop any relation tables if they exist
            for field in &class_definition.fields {
                if let FieldType::ManyToMany = field.field_type {
                    let relation_table_name = format!(
                        "rel_{}_{}",
                        class_definition.entity_type.to_lowercase(),
                        field.name.to_lowercase()
                    );

                    let rel_table_exists = self.check_table_exists(&relation_table_name).await?;
                    if rel_table_exists {
                        log::info!("Dropping relation table: {}", relation_table_name);
                        let drop_sql =
                            format!("DROP TABLE IF EXISTS {} CASCADE", relation_table_name);
                        sqlx::query(&drop_sql)
                            .execute(&self.db_pool)
                            .await
                            .map_err(Error::Database)?;
                    }
                }
            }

            log::info!("Dropping entity table: {}", table_name);
            let drop_sql = format!("DROP TABLE IF EXISTS {} CASCADE", table_name);
            sqlx::query(&drop_sql)
                .execute(&self.db_pool)
                .await
                .map_err(Error::Database)?;
        }

        // Clean up the entity registry
        self.delete_from_entities_registry(&class_definition.entity_type)
            .await?;

        // Now delete the class definition itself
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

        // Execute each statement, continuing on non-fatal errors
        let mut success_count = 0;
        let mut errors = Vec::new();

        for (i, statement) in statements.iter().enumerate() {
            if !statement.is_empty() {
                log::debug!("Executing statement #{}: {}", i + 1, statement);

                // Determine if this is a statement we should continue on error
                // We should continue on errors for:
                // 1. CREATE/DROP with IF NOT EXISTS/IF EXISTS clauses
                // 2. Index operations
                // 3. Creating tables that already exist
                // 4. Adding columns that already exist
                // 5. Index errors due to non-existent columns (handled specially below)
                let is_safe_to_continue = statement.contains("IF NOT EXISTS")
                    || statement.contains("IF EXISTS")
                    || statement.contains("DROP INDEX")
                    || statement.contains("CREATE INDEX")
                    || statement.contains("CREATE TABLE")
                    || statement.contains("ADD COLUMN");

                // Add more detailed diagnostics for critical operations
                if statement.contains("ALTER TABLE") {
                    log::info!("Executing DDL operation: {}", statement);
                }

                match sqlx::query(statement).execute(&self.db_pool).await {
                    Ok(result) => {
                        log::debug!(
                            "Statement #{} executed successfully, rows affected: {}",
                            i + 1,
                            result.rows_affected()
                        );
                        success_count += 1;
                    }
                    Err(e) => {
                        let error_msg = format!("Error executing statement #{}: {}", i + 1, e);
                        log::error!("{}", error_msg);
                        log::error!("Failed statement: {}", statement);

                        // Special handling for index creation on non-existent columns
                        let is_column_not_exist_error = e.to_string().contains("column")
                            && e.to_string().contains("does not exist")
                            && statement.contains("CREATE INDEX");

                        // Store the error with statement details for better diagnosis
                        errors.push(format!("Statement '{}' failed with: {}", statement, e));

                        // For certain statements, we can continue even if they fail
                        if is_safe_to_continue || is_column_not_exist_error {
                            log::warn!("Continuing despite error on statement that uses IF NOT EXISTS/IF EXISTS or is an index operation");
                        } else {
                            return Err(Error::Database(e));
                        }
                    }
                }
            }
        }

        // Clean up unused entity tables after schema application
        self.cleanup_unused_entity_tables().await?;

        if success_count == statements.len() {
            log::info!(
                "Schema applied successfully, all {} statements executed",
                success_count
            );
            Ok(())
        } else if success_count > 0 {
            log::warn!(
                "Schema partially applied: {}/{} statements succeeded",
                success_count,
                statements.len()
            );
            log::warn!("Errors: {}", errors.join("; "));

            // Return success if we had some successful statements and all failures were "safe to continue"
            Ok(())
        } else {
            log::error!("Schema application failed completely");
            Err(Error::InvalidSchema(errors.join("; ")))
        }
    }

    /// Clean up entity tables that don't have corresponding class definitions
    pub async fn cleanup_unused_entity_tables(&self) -> Result<()> {
        log::info!("Starting cleanup of unused entity tables");

        // Get all tables in the database that start with 'entity_'
        let entity_tables: Vec<String> = sqlx::query_scalar(
            "SELECT table_name FROM information_schema.tables 
             WHERE table_schema = 'public' 
             AND table_type = 'BASE TABLE'
             AND table_name LIKE 'entity\\_%'",
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        // Get all class definitions (entity types)
        let class_definitions: Vec<ClassDefinition> =
            sqlx::query_as("SELECT * FROM class_definitions")
                .fetch_all(&self.db_pool)
                .await
                .map_err(Error::Database)?;

        // Create a set of valid entity table names
        let valid_table_names: HashSet<String> = class_definitions
            .iter()
            .map(|def| def.get_table_name())
            .collect();

        // Create a set of protected system tables that should never be dropped
        let protected_tables: HashSet<String> = vec![
            "entities_versions".to_string(),
            "entities_registry".to_string(),
        ]
        .into_iter()
        .collect();

        // Process entity tables
        let mut dropped_tables = 0;

        for table in &entity_tables {
            // Only process entity_* tables that aren't in our valid or protected sets
            if !valid_table_names.contains(table) && !protected_tables.contains(table) {
                log::info!("Dropping unused entity table: {}", table);
                let drop_sql = format!("DROP TABLE IF EXISTS {} CASCADE", table);
                sqlx::query(&drop_sql)
                    .execute(&self.db_pool)
                    .await
                    .map_err(Error::Database)?;
                dropped_tables += 1;
            }
        }

        log::info!(
            "Cleanup complete. Dropped {} unused entity tables",
            dropped_tables
        );
        Ok(())
    }

    /// Update entity table when fields change in a class definition
    pub async fn update_entity_table_for_class_definition(
        &self,
        class_definition: &ClassDefinition,
    ) -> Result<()> {
        // Get the table name from the class definition
        let table_name = class_definition.get_table_name();
        let entity_type = &class_definition.entity_type;

        // Check if the table exists
        let table_exists = self.check_table_exists(&table_name).await?;

        if !table_exists {
            // Create the entity table with just UUID (referenced to registry)
            let create_table_sql = format!(
                "CREATE TABLE IF NOT EXISTS {} (uuid UUID PRIMARY KEY REFERENCES entities_registry(uuid) ON DELETE CASCADE)",
                table_name
            );

            sqlx::query(&create_table_sql)
                .execute(&self.db_pool)
                .await
                .map_err(Error::Database)?;

            log::info!("Created entity table: {}", table_name);
        }

        // Get current columns with their types
        let current_columns = self.get_table_columns_with_types(&table_name).await?;
        log::info!(
            "Current columns in table {}: {:?}",
            table_name,
            current_columns
        );

        let current_column_names: HashSet<String> = current_columns.keys().cloned().collect();

        // Only 'uuid' is a required system column in the entity-specific table
        let system_columns = ["uuid"];

        // Prepare statements for different operations
        let mut column_statements = Vec::new(); // Add standard columns
        let mut drop_column_statements = Vec::new(); // Drop columns
        let mut drop_index_statements = Vec::new(); // Drop indices
        let mut index_statements = Vec::new(); // Create indices

        // Track which columns will exist after all operations
        let mut future_columns = current_column_names.clone();

        // Find columns to be dropped (columns in the table but not mapped to any field)
        for col_name in &current_column_names {
            if system_columns.contains(&col_name.as_str()) {
                continue; // Skip system columns
            }

            // Check if this column is represented by any field
            let column_needed = class_definition.fields.iter().any(|field| {
                let matches = match field.field_type {
                    FieldType::ManyToOne => {
                        // For many-to-one relations, the column name is field_name_uuid
                        format!("{}_uuid", field.name.to_lowercase()) == *col_name
                    }
                    FieldType::ManyToMany => false, // Many-to-many uses separate tables
                    _ => field.name.to_lowercase() == *col_name, // Regular fields
                };

                if matches {
                    log::debug!("Column '{}' matches field '{}'", col_name, field.name);
                }

                matches
            });

            if !column_needed {
                // First, find and drop any indices on the column to be removed
                let drop_index_stmt =
                    format!("DROP INDEX IF EXISTS idx_{}_{}", table_name, col_name);
                log::info!("Column '{}' not needed - will drop index", col_name);
                drop_index_statements.push(drop_index_stmt);

                // Then create the DROP COLUMN statement
                let drop_stmt = format!(
                    "ALTER TABLE {} DROP COLUMN IF EXISTS {}",
                    table_name, col_name
                );
                log::info!("Column '{}' not needed - will drop column", col_name);
                drop_column_statements.push(drop_stmt);

                // Remove this column from our tracking set
                future_columns.remove(col_name);
            } else {
                log::info!("Column '{}' is still needed - keeping", col_name);
            }
        }

        // Now check for columns that need to be added (fields in definition but not in db)
        // Skip system fields as they're now stored in entities_registry
        let registry_fields = [
            "path",
            "created_at",
            "updated_at",
            "created_by",
            "updated_by",
            "published",
            "version",
        ];

        for field in &class_definition.fields {
            // Skip if this is a system field now stored in entities_registry
            if registry_fields.contains(&field.name.as_str()) {
                continue;
            }

            let column_name = match field.field_type {
                FieldType::ManyToOne => {
                    if let Some(_) = &field.validation.target_class {
                        format!("{}_uuid", field.name.to_lowercase())
                    } else {
                        continue; // Skip relation fields without target_class
                    }
                }
                FieldType::ManyToMany => {
                    continue; // Skip many-to-many relationships - they use separate tables
                }
                _ => field.name.to_lowercase(), // Regular fields
            };

            if !current_column_names.contains(&column_name) {
                // Determine SQL type based on field type
                let sql_type = match field.field_type {
                    FieldType::String => "TEXT",
                    FieldType::Integer => "INTEGER",
                    FieldType::Float => "DOUBLE PRECISION",
                    FieldType::Boolean => "BOOLEAN",
                    FieldType::Date => "DATE",
                    FieldType::DateTime => "TIMESTAMP WITH TIME ZONE",
                    FieldType::Json => "JSONB",
                    FieldType::Uuid => "UUID",
                    FieldType::ManyToOne => "UUID",
                    FieldType::Select => "TEXT",
                    FieldType::MultiSelect => "TEXT[]",
                    FieldType::Array => "JSONB",
                    FieldType::Object => "JSONB",
                    FieldType::ManyToMany => continue, // Skip, handled separately
                    FieldType::Text => "TEXT",
                    FieldType::Wysiwyg => "TEXT",
                    FieldType::Image => "TEXT",
                    FieldType::File => "TEXT",
                };

                // Determine if column should be nullable
                let nullable = !field.required;
                let null_constraint = if nullable { "" } else { " NOT NULL" };

                // Create the ALTER TABLE statement to add the column
                let alter_statement = format!(
                    "ALTER TABLE \"{}\" ADD COLUMN {} {}{}",
                    table_name, column_name, sql_type, null_constraint
                );

                log::info!(
                    "Adding column statement for {}: {}",
                    column_name,
                    alter_statement
                );
                column_statements.push(alter_statement);

                // Add this column to our tracking set for future index creation
                future_columns.insert(column_name.clone());
            }

            // Check if this field should be indexed
            if field.indexed {
                // Only create index if the column will exist after all operations
                if future_columns.contains(&column_name) {
                    let index_name = format!("idx_{}_{}", table_name, column_name);

                    // Check if index already exists to avoid duplicates
                    let index_exists = sqlx::query_scalar::<_, bool>(
                        "SELECT EXISTS (
                            SELECT FROM pg_indexes
                            WHERE schemaname = 'public'
                            AND tablename = $1
                            AND indexname = $2
                        )",
                    )
                    .bind(&table_name)
                    .bind(&index_name)
                    .fetch_one(&self.db_pool)
                    .await
                    .map_err(Error::Database)?;

                    if !index_exists {
                        log::info!("Creating index {} for column {}", index_name, column_name);
                        index_statements.push(format!(
                            "CREATE INDEX IF NOT EXISTS {} ON {} ({})",
                            index_name, table_name, column_name
                        ));
                    }
                } else {
                    log::warn!(
                        "Cannot create index for field {} as column {} won't exist after update",
                        field.name,
                        column_name
                    );
                }
            }
        }

        // Handle many-to-many relationship tables
        for field in &class_definition.fields {
            if let FieldType::ManyToMany = field.field_type {
                if let Some(target_class) = &field.validation.target_class {
                    // Format for relation table: rel_[entity_type]_[field_name]
                    let relation_table = format!(
                        "rel_{}_{}",
                        class_definition.entity_type.to_lowercase(),
                        field.name.to_lowercase()
                    );

                    // Check if the relation table exists
                    let relation_table_exists = self.check_table_exists(&relation_table).await?;

                    if !relation_table_exists {
                        log::info!("Creating relation table {}", relation_table);

                        // Create the relation table
                        sqlx::query(&format!(
                            "CREATE TABLE {} (
                                id SERIAL PRIMARY KEY,
                                {}_uuid UUID NOT NULL REFERENCES {} (uuid) ON DELETE CASCADE,
                                {}_uuid UUID NOT NULL,
                                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                                UNIQUE({}_uuid, {}_uuid)
                            )",
                            relation_table,
                            class_definition.entity_type.to_lowercase(),
                            table_name,
                            target_class.to_lowercase(),
                            class_definition.entity_type.to_lowercase(),
                            target_class.to_lowercase()
                        ))
                        .execute(&self.db_pool)
                        .await
                        .map_err(Error::Database)?;

                        // Create indices for the relation table
                        sqlx::query(&format!(
                            "CREATE INDEX idx_{}_{} ON {} ({}_uuid)",
                            relation_table,
                            class_definition.entity_type.to_lowercase(),
                            relation_table,
                            class_definition.entity_type.to_lowercase()
                        ))
                        .execute(&self.db_pool)
                        .await
                        .map_err(Error::Database)?;

                        sqlx::query(&format!(
                            "CREATE INDEX idx_{}_{} ON {} ({}_uuid)",
                            relation_table,
                            target_class.to_lowercase(),
                            relation_table,
                            target_class.to_lowercase()
                        ))
                        .execute(&self.db_pool)
                        .await
                        .map_err(Error::Database)?;
                    }
                }
            }
        }

        // Execute all prepared statements
        for drop_index_stmt in &drop_index_statements {
            log::info!("Executing: {}", drop_index_stmt);
            match sqlx::query(drop_index_stmt).execute(&self.db_pool).await {
                Ok(_) => {}
                Err(e) => log::warn!("Error dropping index: {}", e),
            }
        }

        // First drop the view if it exists to ensure a clean recreation
        // The view must be dropped BEFORE dropping columns to avoid dependency constraints
        let view_name = format!("entity_{}_view", entity_type);
        let drop_view_sql = format!("DROP VIEW IF EXISTS {} CASCADE", view_name);

        match sqlx::query(&drop_view_sql).execute(&self.db_pool).await {
            Ok(_) => log::info!("Dropped view: {} before table modifications", view_name),
            Err(e) => {
                log::error!("Error dropping view: {} - {}", view_name, e);
                return Err(Error::Database(e));
            }
        }

        // Now execute drop column statements after the view has been dropped
        for drop_stmt in &drop_column_statements {
            log::info!("Executing: {}", drop_stmt);
            match sqlx::query(drop_stmt).execute(&self.db_pool).await {
                Ok(_) => {
                    log::info!("Successfully dropped column with: {}", drop_stmt);
                }
                Err(e) => {
                    // Instead of just warning, handle specific error cases
                    let error_msg = e.to_string();

                    // For "column does not exist" errors, we can safely continue
                    if error_msg.contains("column") && error_msg.contains("does not exist") {
                        log::warn!("Column already doesn't exist, continuing: {}", error_msg);
                    } else {
                        // For other errors, return an error to prevent view recreation with incorrect columns
                        log::error!(
                            "Failed to drop column: {} - Error: {}",
                            drop_stmt,
                            error_msg
                        );
                        return Err(Error::Database(e));
                    }
                }
            }
        }

        // Verify that columns were actually dropped before proceeding
        if !drop_column_statements.is_empty() {
            // Get the current columns again to verify drops were successful
            let updated_columns = self.get_table_columns_with_types(&table_name).await?;
            log::info!("Columns after dropping: {:?}", updated_columns);

            // Check if any columns that should have been dropped still exist
            for drop_stmt in &drop_column_statements {
                // Extract column name from the DROP statement
                let parts: Vec<&str> = drop_stmt.split_whitespace().collect();
                if parts.len() >= 6 {
                    let col_name = parts[5].trim_end_matches(|c| c == ';' || c == ',' || c == ' ');
                    if updated_columns.contains_key(col_name) {
                        log::error!("Column {} was not dropped properly", col_name);
                        return Err(Error::Database(sqlx::Error::Protocol(format!(
                            "Failed to drop column {}",
                            col_name
                        ))));
                    }
                }
            }
        }

        // Add columns and create indexes
        for column_stmt in &column_statements {
            log::info!("Executing: {}", column_stmt);
            match sqlx::query(column_stmt).execute(&self.db_pool).await {
                Ok(_) => {}
                Err(e) => log::warn!("Error adding column: {}", e),
            }
        }

        for index_stmt in &index_statements {
            log::info!("Executing: {}", index_stmt);
            match sqlx::query(index_stmt).execute(&self.db_pool).await {
                Ok(_) => {}
                Err(e) => log::warn!("Error creating index: {}", e),
            }
        }

        // After all table modifications are complete, create the view
        // using the create_entity_view function
        // This function dynamically builds the view excluding duplicate uuid columns
        let refresh_view_sql = format!("SELECT create_entity_view('{}')", entity_type);

        match sqlx::query(&refresh_view_sql).execute(&self.db_pool).await {
            Ok(_) => log::info!("Created view: {}", view_name),
            Err(e) => {
                log::error!("Error creating view: {} - {}", view_name, e);
                return Err(Error::Database(e));
            }
        }

        // Verify the view has been correctly created with expected columns
        let verify_sql = format!(
            "SELECT column_name FROM information_schema.columns 
             WHERE table_schema = 'public' AND table_name = '{}'
             ORDER BY ordinal_position",
            view_name
        );

        let view_columns: Vec<(String,)> = sqlx::query_as(&verify_sql)
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| {
                log::error!("Error verifying view columns: {}", e);
                Error::Database(e)
            })?;

        log::info!(
            "View {} created with columns: {:?}",
            view_name,
            view_columns.iter().map(|c| &c.0).collect::<Vec<&String>>()
        );

        Ok(())
    }

    /// Get column names and their types for a table
    pub async fn get_table_columns_with_types(
        &self,
        table_name: &str,
    ) -> Result<HashMap<String, String>> {
        let query = "
            SELECT column_name, data_type, udt_name
            FROM information_schema.columns 
            WHERE table_schema = 'public' AND table_name = $1
            ORDER BY column_name
        ";

        let columns: Vec<(String, String, String)> = sqlx::query_as(query)
            .bind(table_name.to_lowercase())
            .fetch_all(&self.db_pool)
            .await
            .map_err(Error::Database)?;

        let mut result = HashMap::new();
        for (name, data_type, udt_name) in columns {
            // For custom types, use udt_name instead of data_type
            let type_name = match data_type.as_str() {
                "USER-DEFINED" => udt_name,
                "ARRAY" => format!("{}[]", udt_name.trim_start_matches('_')),
                _ => data_type,
            };
            result.insert(name.to_lowercase(), type_name.to_lowercase());
        }

        Ok(result)
    }

    pub async fn check_table_exists(&self, table_name: &str) -> Result<bool> {
        let result: (bool,) = sqlx::query_as::<_, (bool,)>(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = $1
            )",
        )
        .bind(table_name.to_lowercase())
        .fetch_one(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        Ok(result.0)
    }

    pub async fn count_table_records(&self, table_name: &str) -> Result<i64> {
        let result: (i64,) =
            sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {}", table_name))
                .fetch_one(&self.db_pool)
                .await
                .map_err(Error::Database)?;

        Ok(result.0)
    }

    pub async fn delete_from_entities_registry(&self, entity_type: &str) -> Result<()> {
        sqlx::query("DELETE FROM entities_registry WHERE entity_type = $1")
            .bind(entity_type)
            .execute(&self.db_pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    }

    pub async fn get_by_entity_type(&self, entity_type: &str) -> Result<Option<ClassDefinition>> {
        // Use custom query with explicit type casting
        let class_def = sqlx::query!(
            r#"
            SELECT 
                uuid, entity_type, display_name, description, group_name, 
                allow_children, icon, field_definitions as "field_definitions: serde_json::Value",
                created_at, updated_at, 
                created_by as "created_by: Uuid", updated_by,
                published, version
            FROM class_definitions
            WHERE LOWER(entity_type) = LOWER($1)
            "#,
            entity_type
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(Error::Database)?;

        // Return early if not found
        if class_def.is_none() {
            return Ok(None);
        }

        let class_def = class_def.unwrap();

        // Create schema
        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(class_def.entity_type.clone()),
        );
        let schema = crate::entity::class::schema::Schema::new(properties);

        // Convert to ClassDefinition
        Ok(Some(ClassDefinition {
            uuid: class_def.uuid,
            entity_type: class_def.entity_type,
            display_name: class_def.display_name,
            description: class_def.description,
            group_name: class_def.group_name,
            allow_children: class_def.allow_children,
            icon: class_def.icon,
            fields: serde_json::from_value(class_def.field_definitions)
                .map_err(|e| Error::Serialization(e))?,
            schema,
            created_at: class_def.created_at,
            updated_at: class_def.updated_at,
            created_by: class_def.created_by,
            updated_by: class_def.updated_by,
            published: class_def.published,
            version: class_def.version,
        }))
    }
}
