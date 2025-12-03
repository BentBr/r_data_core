use log::{debug, error};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::dynamic_entity_mapper;
use crate::dynamic_entity_repository_trait::FilterEntitiesParams;
use crate::dynamic_entity_utils;
use r_data_core_core::error::Result;
use r_data_core_core::DynamicEntity;

use super::DynamicEntityRepository;

/// Filter entities by field values with advanced options
pub async fn filter_entities_impl(
    repo: &DynamicEntityRepository,
    entity_type: &str,
    params: &FilterEntitiesParams,
) -> Result<Vec<DynamicEntity>> {
    let view_name = dynamic_entity_utils::get_view_name(entity_type);

    // Build query prefix with field selection
    let query_prefix = build_query_prefix(&view_name, params.fields.as_ref());

    // Build WHERE clause with filters and search
    let (mut query, _param_index) = build_where_clause(
        query_prefix,
        params.filters.as_ref(),
        params.search.as_ref(),
    );

    // Add sort and pagination
    add_sort_and_pagination(&mut query, params.sort.as_ref(), params.limit, params.offset);

    debug!("Executing filter query: {query}");

    // Get the entity definition for mapping
    let entity_def = dynamic_entity_utils::get_entity_definition(
        &repo.pool,
        entity_type,
        repo.cache_manager.clone(),
    )
    .await?;

    // Execute query with proper parameter binding
    let rows = execute_filter_query(
        &query,
        &repo.pool,
        params.filters.as_ref(),
        params.search.as_ref(),
    )
    .await?;

    // Map rows to DynamicEntity objects
    let entities: Vec<DynamicEntity> = rows
        .iter()
        .map(|row| dynamic_entity_mapper::map_row_to_entity(row, entity_type, &entity_def))
        .collect();

    Ok(entities)
}

/// Build query prefix with field selection
fn build_query_prefix(view_name: &str, fields: Option<&Vec<String>>) -> String {
    fields.map_or_else(
        || format!("SELECT * FROM {view_name}"),
        |field_list| {
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
                "parent_uuid".to_string(),
            ];

            // Add requested fields
            for field in field_list {
                if !selected_fields.contains(field) {
                    selected_fields.push(field.clone());
                }
            }

            format!("SELECT {} FROM {view_name}", selected_fields.join(", "))
        },
    )
}

/// Build WHERE clause with filters and search
fn build_where_clause(
    mut query: String,
    filters: Option<&std::collections::HashMap<String, JsonValue>>,
    search: Option<&(String, Vec<String>)>,
) -> (String, i32) {
    let mut param_index = 1;

    // Add filter conditions if provided
    if let Some(filter_map) = filters {
        if !filter_map.is_empty() {
            query.push_str(" WHERE ");
            let mut is_first = true;

            for (field, value) in filter_map {
                if !is_first {
                    query.push_str(" AND ");
                }

                param_index = add_filter_condition(&mut query, field, value, param_index);
                is_first = false;
            }
        }
    }

    // Add search condition if provided
    if let Some((_search_term, search_fields)) = search {
        if !search_fields.is_empty() {
            if filters.is_none_or(std::collections::HashMap::is_empty) {
                query.push_str(" WHERE ");
            } else {
                query.push_str(" AND ");
            }

            let search_conditions: Vec<String> = search_fields
                .iter()
                .map(|field| {
                    let condition = format!("{field} ILIKE ${param_index}");
                    param_index += 1;
                    condition
                })
                .collect();

            // Note: search_term is used in execute_filter_query for binding

            if !search_conditions.is_empty() {
                query.push('(');
                query.push_str(&search_conditions.join(" OR "));
                query.push(')');
            }
        }
    }

    (query, param_index)
}

/// Add a single filter condition to the query
fn add_filter_condition(
    query: &mut String,
    field: &str,
    value: &JsonValue,
    param_index: i32,
) -> i32 {
    // Special handling for path-based filters
    if field == "path_prefix" {
        #[allow(clippy::format_push_string)]
        {
            query.push_str(&format!("path LIKE ${param_index} || '/%'"));
        }
        param_index + 1
    } else if field == "path_equals" || field == "path" {
        #[allow(clippy::format_push_string)]
        {
            query.push_str(&format!("path = ${param_index}"));
        }
        param_index + 1
    } else if value == &JsonValue::Null {
        #[allow(clippy::format_push_string)]
        {
            query.push_str(&format!("{field} IS NULL"));
        }
        param_index
    } else {
        #[allow(clippy::format_push_string)]
        {
            query.push_str(&format!("{field} = ${param_index}"));
        }
        param_index + 1
    }
}

/// Add sort and pagination to query
fn add_sort_and_pagination(
    query: &mut String,
    sort: Option<&(String, String)>,
    limit: i64,
    offset: i64,
) {
    // Add sort if provided
    if let Some((field, direction)) = sort {
        // Sanitize the direction to prevent SQL injection
        let sanitized_direction = match direction.to_uppercase().as_str() {
            "ASC" => "ASC",
            _ => "DESC",
        };

        #[allow(clippy::format_push_string)]
        {
            query.push_str(&format!(" ORDER BY {field} {sanitized_direction}"));
        }
    } else {
        // Default sort
        query.push_str(" ORDER BY created_at DESC");
    }

    // Add pagination
    #[allow(clippy::format_push_string)]
    {
        query.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));
    }
}

/// Execute the filter query with proper parameter binding
async fn execute_filter_query(
    query: &str,
    pool: &sqlx::PgPool,
    filters: Option<&std::collections::HashMap<String, JsonValue>>,
    search: Option<&(String, Vec<String>)>,
) -> Result<Vec<sqlx::postgres::PgRow>> {
    let mut sql = sqlx::query(query);

    // Bind filter parameters with proper types
    if let Some(filter_map) = filters {
        for (field, value) in filter_map {
            // Special handling for parent_uuid - bind as UUID type
            if field == "parent_uuid" {
                if let Some(uuid_str) = value.as_str() {
                    if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                        sql = sql.bind(uuid);
                    } else {
                        sql = sql.bind(uuid_str);
                    }
                }
                continue;
            }
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
                    // Skip binding for NULL values
                }
                _ => sql = sql.bind(value.to_string()),
            }
        }
    }

    // Bind search parameters - bind once for each search field
    if let Some((search_term, search_fields)) = search {
        for _field in search_fields {
            sql = sql.bind(format!("%{search_term}%"));
        }
    }

    let rows = sql.fetch_all(pool).await.map_err(|e| {
        error!("Database error: {e}");
        r_data_core_core::error::Error::Database(e)
    })?;

    Ok(rows)
}
