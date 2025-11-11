use log::{debug, error, warn};
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::cache::CacheManager;
use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::dynamic_entity::mapper;
use crate::entity::dynamic_entity::repository_trait::DynamicEntityRepositoryTrait;
use crate::entity::dynamic_entity::utils;
use crate::error::{Error, Result};

/// Repository for managing dynamic entities
pub struct DynamicEntityRepository {
    /// Database connection pool
    pub pool: PgPool,
    /// Cache manager for entity definitions
    pub cache_manager: Option<Arc<CacheManager>>,
}

impl DynamicEntityRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache_manager: None,
        }
    }

    /// Create a new repository instance with cache manager
    pub fn with_cache(pool: PgPool, cache_manager: Arc<CacheManager>) -> Self {
        Self {
            pool,
            cache_manager: Some(cache_manager),
        }
    }

    /// Create a new dynamic entity
    pub async fn create(&self, entity: &DynamicEntity) -> Result<()> {
        // Get the entity definition to validate against
        let _entity_def = utils::get_entity_definition(
            &self.pool,
            &entity.entity_type,
            self.cache_manager.clone(),
        )
        .await?;

        // Validate the entity against the entity definition
        entity.validate()?;

        // Extract UUID from the entity
        let uuid = utils::extract_uuid_from_entity_field_data(&entity.field_data, "uuid")
            .ok_or_else(|| Error::Validation("Entity is missing a valid UUID".to_string()))?;

        // Extract the path (default root) and mandatory key
        let mut path = entity
            .field_data
            .get("path")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "/".to_string());

        let key = entity
            .field_data
            .get("entity_key")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("Missing required field 'entity_key'".to_string()))?;

        // Resolve parent_uuid, and validate/normalize path consistency
        let mut resolved_parent_uuid =
            utils::extract_uuid_from_entity_field_data(&entity.field_data, "parent_uuid");

        // If no explicit parent given, try to infer it from the provided path by
        // checking if there exists an entity whose full path (parent.path + '/' + parent.key)
        // equals the provided path
        if resolved_parent_uuid.is_none() && path != "/" {
            if let Some((parent_uuid_found, _parent_path, _parent_key)) = sqlx::query_as::<_, (Uuid, String, String)>(
                "SELECT uuid, path, entity_key FROM entities_registry \
                 WHERE (CASE WHEN path = '/' THEN '/' || entity_key ELSE path || '/' || entity_key END) = $1 \
                 LIMIT 1"
            )
            .bind(&path)
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::Database)?
            {
                resolved_parent_uuid = Some(parent_uuid_found);
            }
        }

        // Validate parent/path when we have a parent (explicit or inferred)
        if let Some(parent_uuid_val) = resolved_parent_uuid {
            // Fetch parent entity to validate path
            let parent_result: Option<(String, String)> =
                sqlx::query_as("SELECT path, entity_key FROM entities_registry WHERE uuid = $1")
                    .bind(parent_uuid_val)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(Error::Database)?;

            if let Some((parent_path, parent_key)) = parent_result {
                let expected_path = if parent_path.ends_with('/') {
                    format!("{}{}", parent_path, parent_key)
                } else {
                    format!("{}/{}", parent_path, parent_key)
                };

                // Normalize the path to the expected parent path
                if path != expected_path {
                    path = expected_path;
                }
            } else {
                return Err(Error::Validation("Parent entity not found".to_string()));
            }
        }

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // First, insert into entities_registry (include parent_uuid)
        let registry_query = "
            INSERT INTO entities_registry
                (uuid, entity_type, path, entity_key, created_at, updated_at, created_by, updated_by, published, version, parent_uuid)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ";

        // Extract metadata fields or use defaults
        let created_at = entity
            .field_data
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(|s| {
                OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| OffsetDateTime::now_utc())
            })
            .unwrap_or_else(OffsetDateTime::now_utc);

        let updated_at = entity
            .field_data
            .get("updated_at")
            .and_then(|v| v.as_str())
            .map(|s| {
                OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| OffsetDateTime::now_utc())
            })
            .unwrap_or_else(OffsetDateTime::now_utc);

        let created_by =
            utils::extract_uuid_from_entity_field_data(&entity.field_data, "created_by");

        let updated_by =
            utils::extract_uuid_from_entity_field_data(&entity.field_data, "updated_by");

        let published = entity
            .field_data
            .get("published")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let version = entity
            .field_data
            .get("version")
            .and_then(|v| v.as_i64())
            .unwrap_or(1);

        let result = sqlx::query(registry_query)
            .bind(uuid)
            .bind(&entity.entity_type)
            .bind(&path)
            .bind(&key)
            .bind(created_at)
            .bind(updated_at)
            .bind(created_by)
            .bind(updated_by)
            .bind(published)
            .bind(version)
            .bind(resolved_parent_uuid)
            .execute(&mut *tx)
            .await;

        // Map unique violations on (path,key) to a conflict error
        if let Err(e) = result {
            if let sqlx::Error::Database(db_err) = &e {
                // Postgres unique_violation code
                if db_err.code().as_deref() == Some("23505") {
                    return Err(Error::ValidationFailed(
                        "An entity with the same key already exists in this path".to_string(),
                    ));
                }
            }
            return Err(Error::Database(e));
        }

        // Then, insert custom fields into the entity-specific table
        let table_name = utils::get_table_name(&entity.entity_type);

        // Get column names for this table
        let columns_result = sqlx::query(
            "SELECT column_name
             FROM information_schema.columns
             WHERE table_schema = 'public' AND table_name = $1",
        )
        .bind(&table_name)
        .fetch_all(&mut *tx)
        .await?;

        // Extract column names
        let valid_columns: Vec<String> = columns_result
            .iter()
            .map(|row| {
                row.try_get::<String, _>("column_name")
                    .unwrap_or_default()
                    .to_lowercase()
            })
            .collect();

        // Registry fields that should not be included in the entity table
        let registry_fields = [
            "entity_type",
            "path",
            "created_at",
            "updated_at",
            "created_by",
            "updated_by",
            "published",
            "version",
        ];

        // Build query for entity-specific table
        let mut columns = vec!["uuid".to_string()];
        let mut values = vec![format!("'{}'", uuid)];

        // Add entity-specific fields
        for (key, value) in &entity.field_data {
            if registry_fields.contains(&key.as_str()) || key == "uuid" {
                continue; // Skip fields that are stored in entities_registry
            }

            let key_lower = key.to_lowercase();
            if valid_columns.contains(&key_lower) {
                // Database columns are lowercase, so use lowercase for column name
                columns.push(key_lower);

                // Format the value appropriately based on its type
                let value_str = match value {
                    JsonValue::String(s) => format!("'{}'", s.replace("'", "''")),
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::Bool(b) => {
                        if *b {
                            "TRUE".to_string()
                        } else {
                            "FALSE".to_string()
                        }
                    }
                    JsonValue::Null => "NULL".to_string(),
                    _ => format!("'{}'", value.to_string().replace("'", "''")), // For complex types
                };

                values.push(value_str);
            }
        }

        // Create the INSERT statement for the entity table
        if columns.len() > 1 {
            // If we have more than just the UUID
            let query = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                columns.join(", "),
                values.join(", ")
            );

            sqlx::query(&query).execute(&mut *tx).await?;
        } else {
            // If we only have the UUID, just insert that
            sqlx::query(&format!("INSERT INTO {} (uuid) VALUES ($1)", table_name))
                .bind(uuid)
                .execute(&mut *tx)
                .await?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    /// Update an existing dynamic entity
    pub async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        // Validate the entity against the entity definition
        entity.validate()?;

        // Extract UUID from the entity
        let uuid = utils::extract_uuid_from_entity_field_data(&entity.field_data, "uuid")
            .ok_or_else(|| Error::Validation("Entity is missing a valid UUID".to_string()))?;

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // 1. Update entities_registry table
        // Collect update fields with their proper types
        let mut update_clauses = Vec::new();
        let mut param_index = 1;
        let mut path_param: Option<String> = None;
        let mut entity_key_param: Option<String> = None;
        let mut published_param: Option<bool> = None;
        let mut updated_by_param: Option<uuid::Uuid> = None;

        // Extract metadata fields for update with proper types
        if let Some(path) = entity.field_data.get("path").and_then(|v| v.as_str()) {
            update_clauses.push(format!("path = ${}", param_index));
            path_param = Some(path.to_string());
            param_index += 1;
        }

        // Include optional key update
        if let Some(entity_key) = entity.field_data.get("entity_key").and_then(|v| v.as_str()) {
            update_clauses.push(format!("entity_key = ${}", param_index));
            entity_key_param = Some(entity_key.to_string());
            param_index += 1;
        }

        if let Some(published) = entity.field_data.get("published").and_then(|v| v.as_bool()) {
            update_clauses.push(format!("published = ${}", param_index));
            published_param = Some(published);
            param_index += 1;
        }

        let updated_by =
            utils::extract_uuid_from_entity_field_data(&entity.field_data, "updated_by");

        if let Some(item) = updated_by {
            update_clauses.push(format!("updated_by = ${}", param_index));
            updated_by_param = Some(item);
            param_index += 1;
        }

        // Get the current entity_type from the registry to avoid stale WHERE clauses
        let current_entity_type = sqlx::query_scalar::<_, Option<String>>(
            "SELECT entity_type FROM entities_registry WHERE uuid = $1",
        )
        .bind(uuid)
        .fetch_one(&mut *tx)
        .await?;

        // Always update timestamp and increment version
        let update_registry_query = if update_clauses.is_empty() {
            // Update the timestamp and version
            String::from(
                "UPDATE entities_registry SET updated_at = NOW(), version = version + 1
                WHERE uuid = $1",
            )
        } else {
            // uuid comes after the set clause params
            let uuid_pos = param_index;
            format!(
                "UPDATE entities_registry SET {}, updated_at = NOW(), version = version + 1
                    WHERE uuid = ${}",
                update_clauses.join(", "),
                uuid_pos
            )
        };

        // Create a query builder
        let mut registry_query = sqlx::query(&update_registry_query);

        // Bind values for the set clauses with proper types (in parameter order)
        if let Some(path) = path_param {
            registry_query = registry_query.bind(path);
        }
        if let Some(entity_key) = entity_key_param {
            registry_query = registry_query.bind(entity_key);
        }
        if let Some(published) = published_param {
            registry_query = registry_query.bind(published);
        }
        if let Some(updated_by) = updated_by_param {
            registry_query = registry_query.bind(updated_by);
        }

        // Always bind UUID
        registry_query = registry_query.bind(uuid);

        // Execute the registry update and map unique violations
        let res = registry_query.execute(&mut *tx).await;
        if let Err(e) = res {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.code().as_deref() == Some("23505") {
                    return Err(Error::ValidationFailed(
                        "An entity with the same key already exists in this path".to_string(),
                    ));
                }
            }
            return Err(Error::Database(e));
        }

        // 2. Update entity-specific table
        // Use current_entity_type from the registry, not entity.entity_type
        // This ensures we're updating the correct table even if entity was created as different type
        let current_table_name = if let Some(ref current_type) = current_entity_type {
            utils::get_table_name(current_type)
        } else {
            return Err(Error::Database(sqlx::Error::RowNotFound));
        };
        let table_name = current_table_name;

        // Get column names for this table
        let columns_result = sqlx::query(
            "SELECT column_name
             FROM information_schema.columns
             WHERE table_schema = 'public' AND table_name = $1",
        )
        .bind(&table_name)
        .fetch_all(&mut *tx)
        .await?;

        // Extract column names
        let valid_columns: Vec<String> = columns_result
            .iter()
            .map(|row| {
                row.try_get::<String, _>("column_name")
                    .unwrap_or_default()
                    .to_lowercase()
            })
            .collect();

        // Registry fields that should not be included in the entity table
        let registry_fields = [
            "entity_type",
            "path",
            "created_at",
            "updated_at",
            "created_by",
            "updated_by",
            "published",
            "version",
        ];

        // Build SET clauses for entity-specific fields with proper parameterization
        let mut set_clauses = Vec::new();
        let mut entity_params: Vec<(i32, JsonValue)> = Vec::new();
        let mut param_index = 1;

        for (key, value) in &entity.field_data {
            if registry_fields.contains(&key.as_str()) || key == "uuid" {
                continue; // Skip fields that are stored in entities_registry
            }

            let key_lower = key.to_lowercase();
            if valid_columns.contains(&key_lower) {
                // Database columns are lowercase, so use lowercase for column name
                set_clauses.push(format!("{} = ${}", key_lower, param_index));
                entity_params.push((param_index, value.clone()));
                param_index += 1;
            }
        }

        // Execute the entity update if we have SET clauses
        if !set_clauses.is_empty() {
            // The UUID is the last parameter
            let uuid_pos = param_index;
            let update_entity_query = format!(
                "UPDATE {} SET {} WHERE uuid = ${}",
                table_name,
                set_clauses.join(", "),
                uuid_pos
            );

            let mut entity_query = sqlx::query(&update_entity_query);

            // Bind entity-specific field values with proper types
            for (_, json_value) in &entity_params {
                if let Some(bool_val) = json_value.as_bool() {
                    entity_query = entity_query.bind(bool_val);
                } else if let Some(s) = json_value.as_str() {
                    entity_query = entity_query.bind(s);
                } else if let Some(n) = json_value.as_i64() {
                    entity_query = entity_query.bind(n);
                } else if let Some(n) = json_value.as_f64() {
                    entity_query = entity_query.bind(n);
                } else if json_value.is_null() {
                    entity_query = entity_query.bind(None::<String>);
                } else {
                    // Fallback: bind as JSON string representation
                    entity_query = entity_query.bind(json_value.to_string());
                }
            }

            // Always bind UUID
            entity_query = entity_query.bind(uuid);

            entity_query.execute(&mut *tx).await?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    /// Filter entities by field values with advanced options
    async fn filter_entities(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        filters: Option<HashMap<String, JsonValue>>,
        search: Option<(String, Vec<String>)>,
        sort: Option<(String, String)>,
        fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        // Get the view name using the correct format
        let view_name = utils::get_view_name(entity_type);

        // Start building the query with field selection
        let query_prefix = if let Some(field_list) = &fields {
            // Always include system fields
            let mut selected_fields = vec![
                "uuid".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
                "created_by".to_string(),
                "updated_by".to_string(),
                "published".to_string(),
                "version".to_string(),
                "path".to_string(),
            ];

            // Add requested fields
            for field in field_list {
                if !selected_fields.contains(field) {
                    selected_fields.push(field.clone());
                }
            }

            format!("SELECT {} FROM {}", selected_fields.join(", "), view_name)
        } else {
            format!("SELECT * FROM {}", view_name)
        };

        // Base WHERE clause - no need to filter by entity_type as each view only contains one type
        let mut query = format!("{}", query_prefix);
        let mut bind_values: Vec<String> = vec![];
        let mut param_index = 1;

        // Add filter conditions if provided
        if let Some(filter_map) = &filters {
            if !filter_map.is_empty() {
                query.push_str(" WHERE ");
                let mut is_first = true;

                for (field, value) in filter_map {
                    if !is_first {
                        query.push_str(" AND ");
                    }

                    // Special handling for path-based filters
                    if field == "path_prefix" {
                        // Items under the given prefix (recursive)
                        query.push_str(&format!("path LIKE ${} || '/%'", param_index));
                        bind_values.push(
                            value
                                .as_str()
                                .unwrap_or("")
                                .trim_end_matches('/')
                                .to_string(),
                        );
                        param_index += 1;
                        is_first = false;
                        continue;
                    } else if field == "path_equals" || field == "path" {
                        // Exact path match
                        query.push_str(&format!("path = ${}", param_index));
                        bind_values.push(value.as_str().unwrap_or("").to_string());
                        param_index += 1;
                        is_first = false;
                        continue;
                    }

                    match value {
                        JsonValue::String(s) => {
                            query.push_str(&format!("{} = ${}", field, param_index));
                            bind_values.push(s.to_string());
                            param_index += 1;
                        }
                        JsonValue::Number(n) => {
                            query.push_str(&format!("{} = ${}", field, param_index));
                            bind_values.push(n.to_string());
                            param_index += 1;
                        }
                        JsonValue::Bool(b) => {
                            query.push_str(&format!("{} = ${}", field, param_index));
                            bind_values.push(b.to_string());
                            param_index += 1;
                        }
                        JsonValue::Null => {
                            query.push_str(&format!("{} IS NULL", field));
                        }
                        _ => {
                            query.push_str(&format!("{} = ${}", field, param_index));
                            bind_values.push(value.to_string());
                            param_index += 1;
                        }
                    }
                    is_first = false;
                }
            }
        }

        // Add search condition if provided
        if let Some((search_term, search_fields)) = &search {
            if !search_fields.is_empty() {
                // If we have no WHERE clause yet, add one
                if bind_values.is_empty() {
                    query.push_str(" WHERE ");
                } else {
                    query.push_str(" AND ");
                }

                let search_conditions: Vec<String> = search_fields
                    .iter()
                    .map(|field| {
                        let condition = format!("{} ILIKE ${}", field, param_index);
                        bind_values.push(format!("%{}%", search_term));
                        param_index += 1;
                        condition
                    })
                    .collect();

                if !search_conditions.is_empty() {
                    query.push_str("(");
                    query.push_str(&search_conditions.join(" OR "));
                    query.push_str(")");
                }
            }
        }

        // Add sort if provided
        if let Some((field, direction)) = &sort {
            // Sanitize the direction to prevent SQL injection
            let sanitized_direction = match direction.to_uppercase().as_str() {
                "ASC" => "ASC",
                _ => "DESC",
            };

            query.push_str(&format!(" ORDER BY {} {}", field, sanitized_direction));
        } else {
            // Default sort
            query.push_str(" ORDER BY created_at DESC");
        }

        // Add pagination
        query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        debug!("Executing filter query: {}", query);

        // Get the entity definition for mapping
        let entity_def =
            utils::get_entity_definition(&self.pool, entity_type, self.cache_manager.clone())
                .await?;

        // Prepare and execute the query with proper parameter binding
        let mut sql = sqlx::query(&query);

        // Bind parameters with proper types
        if let Some(filter_map) = &filters {
            for (_, value) in filter_map {
                match value {
                    JsonValue::String(s) => sql = sql.bind(s),
                    JsonValue::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            sql = sql.bind(i);
                        } else if let Some(f) = n.as_f64() {
                            sql = sql.bind(f);
                        } else {
                            sql = sql.bind(n.to_string());
                        }
                    }
                    JsonValue::Bool(b) => sql = sql.bind(*b),
                    JsonValue::Null => {
                        // NULL values are handled in the query with IS NULL
                        continue;
                    }
                    _ => sql = sql.bind(value.to_string()),
                }
            }
        }

        // Bind search parameters
        if let Some((search_term, _)) = &search {
            sql = sql.bind(format!("%{}%", search_term));
        }

        let rows = sql.fetch_all(&self.pool).await.map_err(|e| {
            error!("Database error: {}", e);
            Error::Database(e)
        })?;

        // Map rows to DynamicEntity objects
        let mut entities = Vec::new();
        for row in rows {
            let entity = mapper::map_row_to_entity(&row, entity_type, &entity_def);
            entities.push(entity);
        }

        Ok(entities)
    }

    /// Count entities of a specific type
    pub async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        // Use the view for this entity type
        let view_name = utils::get_view_name(entity_type);

        // Check if view exists
        let view_exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM information_schema.tables
                WHERE table_schema = 'public'
                AND table_name = $1
            ) AS "exists!"
            "#,
            &view_name
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        if !view_exists {
            return Err(Error::NotFound(format!(
                "Entity type '{}' not found",
                entity_type
            )));
        }

        // Query count
        let query = format!("SELECT COUNT(*) FROM {}", view_name);
        let count: i64 = sqlx::query_scalar(&query)
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(count)
    }

    /// Query entities by parent_uuid or path
    /// Returns entities filtered by either parent_uuid, exact path, or both
    pub async fn query_by_parent_and_path(
        &self,
        entity_type: &str,
        parent_uuid: Option<Uuid>,
        path: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DynamicEntity>> {
        let table_name = utils::get_table_name(entity_type);
        let entity_def =
            utils::get_entity_definition(&self.pool, entity_type, self.cache_manager.clone())
                .await?;

        // Build the query
        let mut query = format!(
            "SELECT e.*, r.path, r.entity_key, r.parent_uuid FROM {} e 
            INNER JOIN entities_registry r ON e.uuid = r.uuid 
            WHERE r.entity_type = $1",
            table_name
        );

        let mut param_index = 2; // Start after entity_type

        // Add parent_uuid filter if provided
        if let Some(_parent_id) = &parent_uuid {
            query.push_str(&format!(" AND r.parent_uuid = ${}", param_index));
            param_index += 1;
        }

        // Add path filter if provided
        if let Some(_p) = path {
            query.push_str(&format!(" AND r.path = ${}", param_index));
            param_index += 1;
        }

        // Add pagination
        query.push_str(" ORDER BY r.created_at DESC LIMIT $");
        query.push_str(&param_index.to_string());
        param_index += 1;
        query.push_str(" OFFSET $");
        query.push_str(&param_index.to_string());

        debug!("Query by parent/path: {}", query);

        // Build the query with proper parameter binding
        let mut sql_query = sqlx::query(&query).bind(entity_type);

        if let Some(parent_id) = &parent_uuid {
            sql_query = sql_query.bind(parent_id);
        }

        if let Some(p) = path {
            sql_query = sql_query.bind(p);
        }

        let rows = sql_query
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!("Error querying entities by parent/path: {:?}", e);
                Error::Database(e)
            })?;

        // Convert rows to DynamicEntity objects
        let entities = rows
            .iter()
            .map(|row| mapper::map_row_to_entity(row, entity_type, &entity_def))
            .collect();

        Ok(entities)
    }

    /// Check if an entity has children
    pub async fn has_children(&self, parent_uuid: &Uuid) -> Result<bool> {
        let exists: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM entities_registry WHERE parent_uuid = $1 LIMIT 1)",
        )
        .bind(parent_uuid)
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(exists.unwrap_or(false))
    }
}

#[async_trait::async_trait]
impl DynamicEntityRepositoryTrait for DynamicEntityRepository {
    async fn create(&self, entity: &DynamicEntity) -> Result<()> {
        self.create(entity).await
    }

    async fn update(&self, entity: &DynamicEntity) -> Result<()> {
        self.update(entity).await
    }

    async fn get_by_type(
        &self,
        entity_type: &str,
        uuid: &Uuid,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Option<DynamicEntity>> {
        debug!("Getting entity of type {} with UUID {}", entity_type, uuid);

        // Get the entity definition to understand entity structure
        let entity_def =
            utils::get_entity_definition(&self.pool, entity_type, self.cache_manager.clone())
                .await?;

        // Get the view name
        let view_name = utils::get_view_name(entity_type);

        // Build the query with field selection
        let query = if let Some(fields) = &exclusive_fields {
            // Always include system fields
            let mut selected_fields = vec![
                "uuid".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
                "created_by".to_string(),
                "updated_by".to_string(),
                "published".to_string(),
                "version".to_string(),
                "path".to_string(),
            ];

            // Add requested fields
            for field in fields {
                if !selected_fields.contains(field) {
                    selected_fields.push(field.clone());
                }
            }

            format!(
                "SELECT {} FROM {} WHERE uuid = $1",
                selected_fields.join(", "),
                view_name
            )
        } else {
            format!("SELECT * FROM {} WHERE uuid = $1", view_name)
        };

        debug!("Query: {}", query);

        let row = sqlx::query(&query)
            .bind(uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Error fetching entity: {:?}", e);
                Error::Database(e)
            })?;

        if let Some(row) = row {
            // Map the row to a DynamicEntity
            let entity = mapper::map_row_to_entity(&row, entity_type, &entity_def);
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }

    async fn get_all_by_type(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        debug!("Getting all entities of type {}", entity_type);

        // Get the entity definition to understand entity structure
        let entity_def =
            utils::get_entity_definition(&self.pool, entity_type, self.cache_manager.clone())
                .await?;

        // Get the view name
        let view_name = utils::get_view_name(entity_type);

        // Build the query with field selection
        let query = if let Some(fields) = &exclusive_fields {
            // Always include system fields
            let mut selected_fields = vec![
                "uuid".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
                "created_by".to_string(),
                "updated_by".to_string(),
                "published".to_string(),
                "version".to_string(),
                "path".to_string(),
            ];

            // Add requested fields
            for field in fields {
                if !selected_fields.contains(field) {
                    selected_fields.push(field.clone());
                }
            }

            format!(
                "SELECT {} FROM {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                selected_fields.join(", "),
                view_name
            )
        } else {
            format!(
                "SELECT * FROM {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                view_name
            )
        };

        debug!("Query: {}", query);

        // Query all entities
        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!("Error fetching entities: {:?}", e);
                Error::Database(e)
            })?;

        // Convert rows to DynamicEntity objects
        let entities = rows
            .iter()
            .map(|row| mapper::map_row_to_entity(row, entity_type, &entity_def))
            .collect();

        Ok(entities)
    }

    async fn delete_by_type(&self, entity_type: &str, uuid: &Uuid) -> Result<()> {
        debug!("Deleting entity of type {} with UUID {}", entity_type, uuid);

        // Get the table name
        let table_name = utils::get_table_name(entity_type);

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // First, delete from the entity-specific table
        let query = format!("DELETE FROM {} WHERE uuid = $1", table_name);

        let result = sqlx::query(&query).bind(uuid).execute(&mut *tx).await;

        // If the entity table doesn't exist, just log a warning
        if let Err(e) = result {
            warn!("Error deleting from {}: {}", table_name, e);
        }

        // Then delete from entities_registry
        sqlx::query("DELETE FROM entities_registry WHERE uuid = $1 AND entity_type = $2")
            .bind(uuid)
            .bind(entity_type)
            .execute(&mut *tx)
            .await
            .map_err(Error::Database)?;

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    async fn filter_entities(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        filters: Option<HashMap<String, JsonValue>>,
        search: Option<(String, Vec<String>)>,
        sort: Option<(String, String)>,
        fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        self.filter_entities(entity_type, limit, offset, filters, search, sort, fields)
            .await
    }

    async fn count_entities(&self, entity_type: &str) -> Result<i64> {
        self.count_entities(entity_type).await
    }
}
