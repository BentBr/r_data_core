use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use chrono;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Column, Row};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::ApiState;
use crate::entity::{ClassDefinition, DynamicEntity};
use crate::error::{Error, Result as RdataResult};
use utoipa::ToSchema;

/// Simplified entity type information
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityTypeInfo {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub entity_count: i64,
    pub field_count: i32,
}

/// Query parameters for entity listing
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityQuery {
    pub filter: Option<HashMap<String, Value>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
}

/// Helper function to convert serde_json::Value
fn convert_value(value: &serde_json::Value) -> serde_json::Value {
    value.clone()
}

/// Register public API routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .wrap(crate::api::middleware::ApiAuth::new())
            .service(list_available_entities)
            .service(get_entity)
            .service(list_entities)
            .service(create_entity)
            .service(update_entity)
            .service(delete_entity)
            .service(query_entities),
    );
}

/// List all available entity types
#[get("/entities")]
async fn list_available_entities(data: web::Data<ApiState>) -> impl Responder {
    let pool = &data.db_pool;

    // Query entities from the entities registry table
    let result = sqlx::query(
        r#"
        SELECT e.name, e.display_name, e.description, e.is_system, cd.uuid as class_definition_uuid
        FROM entities e
        LEFT JOIN class_definitions cd ON e.name = cd.entity_type
        WHERE e.is_system = true OR cd.published = true
        "#,
    )
    .fetch_all(pool)
    .await;

    match result {
        Ok(rows) => {
            let entities: Vec<EntityTypeInfo> = rows
                .iter()
                .map(|row| EntityTypeInfo {
                    name: row.get("name"),
                    display_name: row.get("display_name"),
                    description: row.get("description"),
                    is_system: row.get("is_system"),
                    entity_count: 0, // We'll fill this later
                    field_count: 0,  // We'll fill this later
                })
                .collect();

            HttpResponse::Ok().json(entities)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to fetch entities: {}", e)
        })),
    }
}

/// Get a specific entity by type and UUID
#[get("/{entity_type}/{uuid}")]
async fn get_entity(data: web::Data<ApiState>, path: web::Path<(String, Uuid)>) -> impl Responder {
    let (entity_type, uuid) = path.into_inner();
    let pool = &data.db_pool;

    // Get class definition to understand table structure
    let class_def_result: RdataResult<Option<ClassDefinition>> = sqlx::query_as!(
        ClassDefinition,
        "SELECT * FROM class_definitions WHERE entity_type = $1",
        entity_type
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::Database);

    let class_def = match class_def_result {
        Ok(Some(def)) => Some(Arc::<ClassDefinition>::new(def)),
        Ok(None) => {
            // Check if it's a system entity
            let system_entity_result = sqlx::query!(
                "SELECT name FROM entities WHERE name = $1 AND is_system = true",
                entity_type
            )
            .fetch_optional(pool)
            .await;

            match system_entity_result {
                Ok(Some(_)) => None, // System entity without class definition
                Ok(None) => {
                    return HttpResponse::NotFound().json(serde_json::json!({
                        "error": format!("Entity type '{}' not found", entity_type)
                    }))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Database error: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch class definition: {}", e)
            }))
        }
    };

    // Build table name (same logic as in ClassDefinition::get_table_name)
    let table_name = match &class_def {
        Some(def) => {
            let def_ref: &ClassDefinition = def;
            def_ref.get_table_name()
        }
        None => format!("{}_entities", entity_type.to_lowercase()),
    };

    // Query the entity from its native table
    let query = format!("SELECT * FROM {} WHERE uuid = $1", table_name);

    let row_result = sqlx::query(&query)
        .bind(uuid.to_string())
        .fetch_optional(pool)
        .await;

    match row_result {
        Ok(Some(row)) => {
            // Convert row to HashMap<String, Value>
            let column_names = row
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect::<Vec<_>>();
            let mut data = HashMap::new();

            for column in column_names {
                // Handle different column types
                let value: Value = if column == "custom_fields" {
                    match row.try_get::<Value, _>(&*column) {
                        Ok(v) => v,
                        Err(_) => Value::Object(serde_json::Map::new()),
                    }
                } else {
                    // Try different types
                    if let Ok(val) = row.try_get::<String, _>(&*column) {
                        Value::String(val)
                    } else if let Ok(val) = row.try_get::<i64, _>(&*column) {
                        Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<i32, _>(&*column) {
                        Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<bool, _>(&*column) {
                        Value::Bool(val)
                    } else if let Ok(val) =
                        row.try_get::<chrono::DateTime<chrono::Utc>, _>(&*column)
                    {
                        Value::String(val.to_rfc3339())
                    } else if let Ok(val) = row.try_get::<serde_json::Value, _>(&*column) {
                        val
                    } else {
                        Value::Null
                    }
                };

                data.insert(column, value);
            }

            let entity = DynamicEntity::from_data(entity_type, data, class_def);
            HttpResponse::Ok().json(entity)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("Entity with UUID '{}' not found", uuid)
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to fetch entity: {}", e)
        })),
    }
}

/// List entities of a specific type with filtering
#[get("/{entity_type}")]
async fn list_entities(
    data: web::Data<ApiState>,
    path: web::Path<String>,
    query: web::Query<EntityQuery>,
) -> impl Responder {
    let entity_type = path.into_inner();
    let pool = &data.db_pool;

    // Get class definition
    let class_def_result: RdataResult<Option<ClassDefinition>> = sqlx::query_as!(
        ClassDefinition,
        "SELECT * FROM class_definitions WHERE entity_type = $1",
        entity_type
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::Database);

    // Build table name
    let table_name = match class_def_result {
        Ok(Some(ref def)) => {
            let def_ref: &ClassDefinition = def;
            def_ref.get_table_name()
        }
        Ok(None) => {
            // Check if it's a system entity
            let system_entity_result = sqlx::query!(
                "SELECT name FROM entities WHERE name = $1 AND is_system = true",
                entity_type
            )
            .fetch_optional(pool)
            .await;

            match system_entity_result {
                Ok(Some(_)) => format!("{}_entities", entity_type.to_lowercase()),
                Ok(None) => {
                    return HttpResponse::NotFound().json(serde_json::json!({
                        "error": format!("Entity type '{}' not found", entity_type)
                    }))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Database error: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch class definition: {}", e)
            }))
        }
    };

    // Build where clause from filters
    let mut where_clauses = Vec::new();
    let mut params = Vec::new();

    let query_params = query.into_inner();

    for (i, (key, value)) in query_params.filter.iter().flatten().enumerate() {
        if key == "custom_fields" {
            // Handle custom_fields (JSONB column)
            if let Value::Object(obj) = value {
                for (cf_key, cf_value) in obj {
                    let json_path = format!("custom_fields->'{}'", cf_key);
                    where_clauses.push(format!("{} = ${}", json_path, i + 1));
                    params.push(cf_value.clone());
                }
            }
        } else {
            // Standard field
            where_clauses.push(format!("{} = ${}", key, i + 1));
            params.push(value.clone());
        }
    }

    // Build the query
    let where_clause = if where_clauses.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    // Add sorting
    let sort_clause = match (query_params.sort_by, query_params.sort_direction) {
        (Some(sort_by), Some(sort_order)) => {
            let direction = if sort_order.to_lowercase() == "desc" {
                "DESC"
            } else {
                "ASC"
            };
            format!("ORDER BY {} {}", sort_by, direction)
        }
        (Some(sort_by), None) => format!("ORDER BY {} ASC", sort_by),
        _ => "ORDER BY uuid ASC".to_string(),
    };

    let limit_offset = format!(
        "LIMIT {} OFFSET {}",
        query_params.limit.unwrap_or(20),
        query_params.offset.unwrap_or(0)
    );

    // Count query
    let count_query = format!("SELECT COUNT(*) FROM {} {}", table_name, where_clause);

    // Main query
    let main_query = format!(
        "SELECT * FROM {} {} {} {}",
        table_name, where_clause, sort_clause, limit_offset
    );

    // Execute count query
    let mut count_query_builder = sqlx::query(&count_query);

    // Bind params for count query
    for value in &params {
        match value {
            Value::String(s) => count_query_builder = count_query_builder.bind(s),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    count_query_builder = count_query_builder.bind(i);
                } else if let Some(f) = n.as_f64() {
                    count_query_builder = count_query_builder.bind(f);
                } else {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid number value"
                    }));
                }
            }
            Value::Bool(b) => count_query_builder = count_query_builder.bind(b),
            Value::Null => count_query_builder = count_query_builder.bind::<Option<String>>(None),
            _ => count_query_builder = count_query_builder.bind(value), // For objects and arrays, bind as JSON
        }
    }

    let count_result = count_query_builder.fetch_one(pool).await;

    let total_count = match count_result {
        Ok(row) => row.get::<i64, _>(0),
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to count entities: {}", e)
            }))
        }
    };

    // Execute main query
    let mut main_query_builder = sqlx::query(&main_query);

    // Bind params for main query
    for value in &params {
        match value {
            Value::String(s) => main_query_builder = main_query_builder.bind(s),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    main_query_builder = main_query_builder.bind(i);
                } else if let Some(f) = n.as_f64() {
                    main_query_builder = main_query_builder.bind(f);
                } else {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid number value"
                    }));
                }
            }
            Value::Bool(b) => main_query_builder = main_query_builder.bind(b),
            Value::Null => main_query_builder = main_query_builder.bind::<Option<String>>(None),
            _ => main_query_builder = main_query_builder.bind(value), // For objects and arrays, bind as JSON
        }
    }

    let rows_result = main_query_builder.fetch_all(pool).await;

    match rows_result {
        Ok(rows) => {
            let class_def = match class_def_result {
                Ok(Some(def)) => Some(Arc::new(def.clone())),
                _ => None,
            };

            // Convert rows to DynamicEntity objects
            let entities: Vec<DynamicEntity> = rows
                .iter()
                .map(|row| {
                    // Convert row to HashMap<String, Value>
                    let column_names = row
                        .columns()
                        .iter()
                        .map(|c| c.name().to_string())
                        .collect::<Vec<_>>();
                    let mut data = HashMap::new();

                    for column in column_names {
                        // Handle different column types
                        let value: Value = if column == "custom_fields" {
                            match row.try_get::<Value, _>(&*column) {
                                Ok(v) => v,
                                Err(_) => Value::Object(serde_json::Map::new()),
                            }
                        } else {
                            // Try different types
                            if let Ok(val) = row.try_get::<String, _>(&*column) {
                                Value::String(val)
                            } else if let Ok(val) = row.try_get::<i64, _>(&*column) {
                                Value::Number(val.into())
                            } else if let Ok(val) = row.try_get::<i32, _>(&*column) {
                                Value::Number(val.into())
                            } else if let Ok(val) = row.try_get::<bool, _>(&*column) {
                                Value::Bool(val)
                            } else if let Ok(val) =
                                row.try_get::<chrono::DateTime<chrono::Utc>, _>(&*column)
                            {
                                Value::String(val.to_rfc3339())
                            } else if let Ok(val) = row.try_get::<serde_json::Value, _>(&*column) {
                                val
                            } else {
                                Value::Null
                            }
                        };

                        data.insert(column, value);
                    }

                    DynamicEntity::from_data(entity_type.clone(), data, class_def.clone())
                })
                .collect();

            // Return the result with pagination info
            HttpResponse::Ok().json(serde_json::json!({
                "total": total_count,
                "offset": query_params.offset.unwrap_or(0),
                "limit": query_params.limit.unwrap_or(20),
                "items": entities
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to query entities: {}", e)
        })),
    }
}

/// Create a new entity
#[post("/{entity_type}")]
async fn create_entity(
    data: web::Data<ApiState>,
    path: web::Path<String>,
    entity_data: web::Json<HashMap<String, Value>>,
) -> impl Responder {
    let entity_type = path.into_inner();
    let pool = &data.db_pool;

    // Get class definition to understand table structure
    let class_def_result: RdataResult<Option<ClassDefinition>> = sqlx::query_as!(
        ClassDefinition,
        "SELECT * FROM class_definitions WHERE entity_type = $1",
        entity_type
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::Database);

    let class_def = match class_def_result {
        Ok(Some(def)) => Some(Arc::<ClassDefinition>::new(def)),
        Ok(None) => {
            // Check if it's a system entity
            let system_entity_result = sqlx::query!(
                "SELECT name FROM entities WHERE name = $1 AND is_system = true",
                entity_type
            )
            .fetch_optional(pool)
            .await;

            match system_entity_result {
                Ok(Some(_)) => None, // System entity without class definition
                Ok(None) => {
                    return HttpResponse::NotFound().json(serde_json::json!({
                        "error": format!("Entity type '{}' not found", entity_type)
                    }))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Database error: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch class definition: {}", e)
            }))
        }
    };

    // Create dynamic entity
    let mut entity = DynamicEntity::new(entity_type.clone(), class_def.clone());

    // Validate and populate fields from request data
    for (key, value) in entity_data.into_inner() {
        if let Err(e) = entity.set::<serde_json::Value>(&key, convert_value(&value)) {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid field '{}': {}", key, e)
            }));
        }
    }

    // Build table name
    let table_name = match &class_def {
        Some(def) => {
            let def_ref: &ClassDefinition = def;
            def_ref.get_table_name()
        }
        None => format!("{}_entities", entity_type.to_lowercase()),
    };

    // Build insert query
    let mut columns = Vec::new();
    let mut placeholders = Vec::new();
    let mut values = Vec::new();

    for (i, (key, value)) in entity.data.iter().enumerate() {
        // Skip custom_fields as it's handled separately
        if key == "custom_fields" {
            columns.push(key.clone());
            placeholders.push(format!("${}", i + 1));
            values.push(value.clone());
            continue;
        }

        // Only include fields that are defined in class definition
        if def
            .schema
            .properties
            .get("properties")
            .and_then(|p| p.as_object())
            .is_some()
            || [
                "uuid",
                "path",
                "created_at",
                "updated_at",
                "published",
                "version",
            ]
            .contains(&key.as_str())
        {
            columns.push(key.clone());
            placeholders.push(format!("${}", i + 1));
            values.push(value.clone());
        }
    }

    let query = format!(
        "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
        table_name,
        columns.join(", "),
        placeholders.join(", ")
    );

    // Prepare the query
    let mut query_builder = sqlx::query(&query);

    // Bind values
    for value in values {
        // Match different value types
        match value {
            Value::String(s) => query_builder = query_builder.bind(s),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    query_builder = query_builder.bind(i);
                } else if let Some(f) = n.as_f64() {
                    query_builder = query_builder.bind(f);
                } else {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid number value"
                    }));
                }
            }
            Value::Bool(b) => query_builder = query_builder.bind(b),
            Value::Null => query_builder = query_builder.bind::<Option<String>>(None),
            _ => query_builder = query_builder.bind(value), // For objects and arrays, bind as JSON
        }
    }

    // Execute the query
    let row_result = query_builder.fetch_optional(pool).await;

    match row_result {
        Ok(Some(row)) => {
            // Convert row to HashMap<String, Value>
            let column_names = row
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect::<Vec<_>>();
            let mut data = HashMap::new();

            for column in column_names {
                // Handle different column types
                let value: Value = if column == "custom_fields" {
                    match row.try_get::<Value, _>(&*column) {
                        Ok(v) => v,
                        Err(_) => Value::Object(serde_json::Map::new()),
                    }
                } else {
                    // Try different types
                    if let Ok(val) = row.try_get::<String, _>(&*column) {
                        Value::String(val)
                    } else if let Ok(val) = row.try_get::<i64, _>(&*column) {
                        Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<i32, _>(&*column) {
                        Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<bool, _>(&*column) {
                        Value::Bool(val)
                    } else if let Ok(val) =
                        row.try_get::<chrono::DateTime<chrono::Utc>, _>(&*column)
                    {
                        Value::String(val.to_rfc3339())
                    } else if let Ok(val) = row.try_get::<serde_json::Value, _>(&*column) {
                        val
                    } else {
                        Value::Null
                    }
                };

                data.insert(column, value);
            }

            let entity = DynamicEntity::from_data(entity_type, data, class_def);
            HttpResponse::Created().json(entity)
        }
        Ok(None) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Entity was created but could not be returned"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to create entity: {}", e)
        })),
    }
}

/// Update an existing entity
#[put("/{entity_type}/{uuid}")]
async fn update_entity(
    data: web::Data<ApiState>,
    path: web::Path<(String, Uuid)>,
    entity_data: web::Json<HashMap<String, Value>>,
) -> impl Responder {
    let (entity_type, uuid) = path.into_inner();
    let pool = &data.db_pool;

    // First, get the existing entity
    //let existing_entity_response = get_entity(
    //    data.clone(),
    //    web::Path::from((entity_type.clone(), uuid))
    //).await;

    // Get entity directly from the database instead of calling the handler
    let pool = &data.db_pool;

    // Get class definition
    let class_def_result: RdataResult<Option<ClassDefinition>> = sqlx::query_as!(
        ClassDefinition,
        "SELECT * FROM class_definitions WHERE entity_type = $1",
        entity_type
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::Database);

    let class_def = match class_def_result {
        Ok(Some(def)) => Some(Arc::<ClassDefinition>::new(def)),
        Ok(None) => {
            // Check if it's a system entity
            let system_entity_result = sqlx::query!(
                "SELECT name FROM entities WHERE name = $1 AND is_system = true",
                entity_type
            )
            .fetch_optional(pool)
            .await;

            match system_entity_result {
                Ok(Some(_)) => None, // System entity without class definition
                Ok(None) => {
                    return HttpResponse::NotFound().json(serde_json::json!({
                        "error": format!("Entity type '{}' not found", entity_type)
                    }))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Database error: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch class definition: {}", e)
            }))
        }
    };

    // Build table name
    let table_name = match &class_def {
        Some(ref def) => {
            let def_ref: &ClassDefinition = def;
            def_ref.get_table_name()
        }
        None => format!("{}_entities", entity_type.to_lowercase()),
    };

    // Query the entity from its native table
    let query = format!("SELECT * FROM {} WHERE uuid = $1", table_name);

    let row_result = sqlx::query(&query)
        .bind(uuid.to_string())
        .fetch_optional(pool)
        .await;

    // Check if entity exists
    let existing_entity = match row_result {
        Ok(Some(row)) => {
            // Convert row to HashMap<String, Value>
            let column_names = row
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect::<Vec<_>>();
            let mut data = HashMap::new();

            for column in column_names {
                // Handle different column types
                let value: Value = if column == "custom_fields" {
                    match row.try_get::<Value, _>(&*column) {
                        Ok(v) => v,
                        Err(_) => Value::Object(serde_json::Map::new()),
                    }
                } else {
                    // Try different types
                    if let Ok(val) = row.try_get::<String, _>(&*column) {
                        Value::String(val)
                    } else if let Ok(val) = row.try_get::<i64, _>(&*column) {
                        Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<i32, _>(&*column) {
                        Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<bool, _>(&*column) {
                        Value::Bool(val)
                    } else if let Ok(val) =
                        row.try_get::<chrono::DateTime<chrono::Utc>, _>(&*column)
                    {
                        Value::String(val.to_rfc3339())
                    } else if let Ok(val) = row.try_get::<serde_json::Value, _>(&*column) {
                        val
                    } else {
                        Value::Null
                    }
                };

                data.insert(column, value);
            }

            DynamicEntity::from_data(entity_type.clone(), data, class_def.clone())
        }
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("Entity with UUID '{}' not found", uuid)
            }))
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch entity: {}", e)
            }))
        }
    };

    // Create a new entity based on existing one with updates
    let mut updated_entity = existing_entity;
    updated_entity.definition = class_def.clone();

    // Update fields from request data
    for (key, value) in entity_data.into_inner() {
        // Don't allow changing id, uuid, created_at, or version
        if ["id", "uuid", "created_at", "version"].contains(&key.as_str()) {
            continue;
        }

        if let Err(e) = updated_entity.set::<serde_json::Value>(&key, convert_value(&value)) {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid field '{}': {}", key, e)
            }));
        }
    }

    // Increment version
    updated_entity.increment_version();

    // Build table name
    let table_name = match &class_def {
        Some(def) => {
            let def_ref: &ClassDefinition = def;
            def_ref.get_table_name()
        }
        None => format!("{}_entities", entity_type.to_lowercase()),
    };

    // Build update query
    let mut set_clauses = Vec::new();
    let mut values = Vec::new();

    for (i, (key, value)) in updated_entity.data.iter().enumerate() {
        // Skip id, uuid, created_at as they shouldn't be updated
        if ["id", "uuid", "created_at"].contains(&key.as_str()) {
            continue;
        }

        // Only include fields that are defined in class definition
        if def
            .schema
            .properties
            .get("properties")
            .and_then(|p| p.as_object())
            .is_some()
            || [
                "path",
                "updated_at",
                "published",
                "version",
                "custom_fields",
            ]
            .contains(&key.as_str())
        {
            set_clauses.push(format!("{} = ${}", key, i + 1));
            values.push(value.clone());
        }
    }

    let query = format!(
        "UPDATE {} SET {} WHERE uuid = '{}' RETURNING *",
        table_name,
        set_clauses.join(", "),
        uuid
    );

    // Prepare the query
    let mut query_builder = sqlx::query(&query);

    // Bind values
    for value in values {
        // Match different value types
        match value {
            Value::String(s) => query_builder = query_builder.bind(s),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    query_builder = query_builder.bind(i);
                } else if let Some(f) = n.as_f64() {
                    query_builder = query_builder.bind(f);
                } else {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid number value"
                    }));
                }
            }
            Value::Bool(b) => query_builder = query_builder.bind(b),
            Value::Null => query_builder = query_builder.bind::<Option<String>>(None),
            _ => query_builder = query_builder.bind(value), // For objects and arrays, bind as JSON
        }
    }

    // Execute the query
    let row_result = query_builder.fetch_optional(pool).await;

    match row_result {
        Ok(Some(row)) => {
            // Convert row to HashMap<String, Value>
            let column_names = row
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect::<Vec<_>>();
            let mut data = HashMap::new();

            for column in column_names {
                // Handle different column types
                let value: Value = if column == "custom_fields" {
                    match row.try_get::<Value, _>(&*column) {
                        Ok(v) => v,
                        Err(_) => Value::Object(serde_json::Map::new()),
                    }
                } else {
                    // Try different types
                    if let Ok(val) = row.try_get::<String, _>(&*column) {
                        Value::String(val)
                    } else if let Ok(val) = row.try_get::<i64, _>(&*column) {
                        Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<i32, _>(&*column) {
                        Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<bool, _>(&*column) {
                        Value::Bool(val)
                    } else if let Ok(val) =
                        row.try_get::<chrono::DateTime<chrono::Utc>, _>(&*column)
                    {
                        Value::String(val.to_rfc3339())
                    } else if let Ok(val) = row.try_get::<serde_json::Value, _>(&*column) {
                        val
                    } else {
                        Value::Null
                    }
                };

                data.insert(column, value);
            }

            let entity = DynamicEntity::from_data(entity_type, data, class_def);
            HttpResponse::Ok().json(entity)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("Entity with UUID '{}' not found", uuid)
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to update entity: {}", e)
        })),
    }
}

/// Delete an entity
#[delete("/{entity_type}/{uuid}")]
async fn delete_entity(
    data: web::Data<ApiState>,
    path: web::Path<(String, Uuid)>,
) -> impl Responder {
    let (entity_type, uuid) = path.into_inner();
    let pool = &data.db_pool;

    // Get class definition
    let class_def_result: RdataResult<Option<ClassDefinition>> = sqlx::query_as!(
        ClassDefinition,
        "SELECT * FROM class_definitions WHERE entity_type = $1",
        entity_type
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::Database);

    // Build table name
    let table_name = match class_def_result {
        Ok(Some(def)) => {
            let def_ref: &ClassDefinition = def;
            def_ref.get_table_name()
        }
        Ok(None) => {
            // Check if it's a system entity
            let system_entity_result = sqlx::query!(
                "SELECT name FROM entities WHERE name = $1 AND is_system = true",
                entity_type
            )
            .fetch_optional(pool)
            .await;

            match system_entity_result {
                Ok(Some(_)) => format!("{}_entities", entity_type.to_lowercase()),
                Ok(None) => {
                    return HttpResponse::NotFound().json(serde_json::json!({
                        "error": format!("Entity type '{}' not found", entity_type)
                    }))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Database error: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch class definition: {}", e)
            }))
        }
    };

    // Execute the delete query
    let query = format!("DELETE FROM {} WHERE uuid = $1 RETURNING uuid", table_name);

    let result = sqlx::query(&query)
        .bind(uuid.to_string())
        .fetch_optional(pool)
        .await;

    match result {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({
            "message": format!("Entity with UUID '{}' deleted successfully", uuid)
        })),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("Entity with UUID '{}' not found", uuid)
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to delete entity: {}", e)
        })),
    }
}

/// Advanced query for entities with more complex filtering
#[post("/{entity_type}/query")]
async fn query_entities(
    data: web::Data<ApiState>,
    path: web::Path<String>,
    query: web::Json<EntityQuery>,
) -> impl Responder {
    let entity_type = path.into_inner();
    let pool = &data.db_pool;

    // Get class definition
    let class_def_result: RdataResult<Option<ClassDefinition>> = sqlx::query_as!(
        ClassDefinition,
        "SELECT * FROM class_definitions WHERE entity_type = $1",
        entity_type
    )
    .fetch_optional(pool)
    .await
    .map_err(Error::Database);

    // Build table name
    let table_name = match class_def_result {
        Ok(Some(def)) => {
            let def_ref: &ClassDefinition = def;
            def_ref.get_table_name()
        }
        Ok(None) => {
            // Check if it's a system entity
            let system_entity_result = sqlx::query!(
                "SELECT name FROM entities WHERE name = $1 AND is_system = true",
                entity_type
            )
            .fetch_optional(pool)
            .await;

            match system_entity_result {
                Ok(Some(_)) => format!("{}_entities", entity_type.to_lowercase()),
                Ok(None) => {
                    return HttpResponse::NotFound().json(serde_json::json!({
                        "error": format!("Entity type '{}' not found", entity_type)
                    }))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Database error: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch class definition: {}", e)
            }))
        }
    };

    // Build where clause from filters
    let mut where_clauses = Vec::new();
    let mut params = Vec::new();

    let query_params = query.into_inner();

    for (i, (key, value)) in query_params.filter.iter().flatten().enumerate() {
        if key == "custom_fields" {
            // Handle custom_fields (JSONB column)
            if let Value::Object(obj) = value {
                for (cf_key, cf_value) in obj {
                    let json_path = format!("custom_fields->'{}'", cf_key);
                    where_clauses.push(format!("{} = ${}", json_path, i + 1));
                    params.push(cf_value.clone());
                }
            }
        } else {
            // Standard field
            where_clauses.push(format!("{} = ${}", key, i + 1));
            params.push(value.clone());
        }
    }

    // Build the query
    let where_clause = if where_clauses.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    // Add sorting
    let sort_clause = match (query_params.sort_by, query_params.sort_direction) {
        (Some(sort_by), Some(sort_order)) => {
            let direction = if sort_order.to_lowercase() == "desc" {
                "DESC"
            } else {
                "ASC"
            };
            format!("ORDER BY {} {}", sort_by, direction)
        }
        (Some(sort_by), None) => format!("ORDER BY {} ASC", sort_by),
        _ => "ORDER BY uuid ASC".to_string(),
    };

    let limit_offset = format!(
        "LIMIT {} OFFSET {}",
        query_params.limit.unwrap_or(20),
        query_params.offset.unwrap_or(0)
    );

    // Count query
    let count_query = format!("SELECT COUNT(*) FROM {} {}", table_name, where_clause);

    // Main query
    let main_query = format!(
        "SELECT * FROM {} {} {} {}",
        table_name, where_clause, sort_clause, limit_offset
    );

    // Execute count query
    let mut count_query_builder = sqlx::query(&count_query);

    // Bind params for count query
    for value in &params {
        match value {
            Value::String(s) => count_query_builder = count_query_builder.bind(s),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    count_query_builder = count_query_builder.bind(i);
                } else if let Some(f) = n.as_f64() {
                    count_query_builder = count_query_builder.bind(f);
                } else {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid number value"
                    }));
                }
            }
            Value::Bool(b) => count_query_builder = count_query_builder.bind(b),
            Value::Null => count_query_builder = count_query_builder.bind::<Option<String>>(None),
            _ => count_query_builder = count_query_builder.bind(value), // For objects and arrays, bind as JSON
        }
    }

    let count_result = count_query_builder.fetch_one(pool).await;

    let total_count = match count_result {
        Ok(row) => row.get::<i64, _>(0),
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to count entities: {}", e)
            }))
        }
    };

    // Execute main query
    let mut main_query_builder = sqlx::query(&main_query);

    // Bind params for main query
    for value in &params {
        match value {
            Value::String(s) => main_query_builder = main_query_builder.bind(s),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    main_query_builder = main_query_builder.bind(i);
                } else if let Some(f) = n.as_f64() {
                    main_query_builder = main_query_builder.bind(f);
                } else {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid number value"
                    }));
                }
            }
            Value::Bool(b) => main_query_builder = main_query_builder.bind(b),
            Value::Null => main_query_builder = main_query_builder.bind::<Option<String>>(None),
            _ => main_query_builder = main_query_builder.bind(value), // For objects and arrays, bind as JSON
        }
    }

    let rows_result = main_query_builder.fetch_all(pool).await;

    match rows_result {
        Ok(rows) => {
            let class_def = match class_def_result {
                Ok(Some(def)) => Some(Arc::new(def.clone())),
                _ => None,
            };

            // Convert rows to DynamicEntity objects
            let entities: Vec<DynamicEntity> = rows
                .iter()
                .map(|row| {
                    // Convert row to HashMap<String, Value>
                    let column_names = row
                        .columns()
                        .iter()
                        .map(|c| c.name().to_string())
                        .collect::<Vec<_>>();
                    let mut data = HashMap::new();

                    for column in column_names {
                        // Handle different column types
                        let value: Value = if column == "custom_fields" {
                            match row.try_get::<Value, _>(&*column) {
                                Ok(v) => v,
                                Err(_) => Value::Object(serde_json::Map::new()),
                            }
                        } else {
                            // Try different types
                            if let Ok(val) = row.try_get::<String, _>(&*column) {
                                Value::String(val)
                            } else if let Ok(val) = row.try_get::<i64, _>(&*column) {
                                Value::Number(val.into())
                            } else if let Ok(val) = row.try_get::<i32, _>(&*column) {
                                Value::Number(val.into())
                            } else if let Ok(val) = row.try_get::<bool, _>(&*column) {
                                Value::Bool(val)
                            } else if let Ok(val) =
                                row.try_get::<chrono::DateTime<chrono::Utc>, _>(&*column)
                            {
                                Value::String(val.to_rfc3339())
                            } else if let Ok(val) = row.try_get::<serde_json::Value, _>(&*column) {
                                val
                            } else {
                                Value::Null
                            }
                        };

                        data.insert(column, value);
                    }

                    DynamicEntity::from_data(entity_type.clone(), data, class_def.clone())
                })
                .collect();

            // Return the result with pagination info
            HttpResponse::Ok().json(serde_json::json!({
                "total": total_count,
                "offset": query_params.offset.unwrap_or(0),
                "limit": query_params.limit.unwrap_or(20),
                "items": entities
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to query entities: {}", e)
        })),
    }
}
