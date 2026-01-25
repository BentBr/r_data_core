#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use async_trait::async_trait;
use log::debug;
use std::fmt::Write;

use crate::dynamic_entity_mapper;
use crate::dynamic_entity_query_repository_trait::DynamicEntityQueryRepositoryTrait;
use crate::dynamic_entity_utils;
use r_data_core_core::error::Result;
use r_data_core_core::public_api::AdvancedEntityQuery;
use r_data_core_core::DynamicEntity;
use sqlx::PgPool;

/// Repository for public API advanced query operations on dynamic entities
///
/// Provides advanced querying capabilities for dynamic entity instances.
pub struct DynamicEntityQueryRepository {
    db_pool: PgPool,
}

impl DynamicEntityQueryRepository {
    /// Create a new dynamic entity query repository
    #[must_use]
    pub const fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Query dynamic entity instances with advanced filtering
    ///
    /// # Errors
    /// Returns an error if the entity type doesn't exist or the query fails
    pub async fn query_entities(
        &self,
        entity_type: &str,
        query: &AdvancedEntityQuery,
    ) -> Result<Vec<DynamicEntity>> {
        // Get entity definition (no caching in this repository)
        let entity_def =
            dynamic_entity_utils::get_entity_definition(&self.db_pool, entity_type, None).await?;

        // Build the query
        let view_name = dynamic_entity_utils::get_view_name(entity_type);

        let mut sql = format!("SELECT * FROM {view_name}");
        let mut params: Vec<String> = Vec::new();

        // Add WHERE clause for filters
        if let Some(filters) = &query.filter {
            if !filters.is_empty() {
                let (where_clause, filter_params) =
                    dynamic_entity_utils::build_where_clause(filters, &entity_def);
                let _ = write!(sql, " WHERE {where_clause}");
                params = filter_params;
            }
        }

        // Add ORDER BY
        if let Some(sort_by) = &query.sort_by {
            let direction = query.sort_direction.as_ref().map_or("ASC", |d| {
                if d.to_uppercase() == "DESC" {
                    "DESC"
                } else {
                    "ASC"
                }
            });
            // Sanitize sort_by to prevent SQL injection (only allow alphanumeric and underscore)
            if sort_by.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let _ = write!(sql, " ORDER BY {sort_by} {direction}");
            } else {
                sql.push_str(" ORDER BY created_at DESC");
            }
        } else {
            sql.push_str(" ORDER BY created_at DESC");
        }

        // Add LIMIT and OFFSET
        let limit = query.limit.unwrap_or(50).min(1000);
        let offset = query.offset.unwrap_or(0).max(0);
        let _ = write!(sql, " LIMIT {limit} OFFSET {offset}");

        debug!("Executing advanced query: {sql}");

        // Execute query with parameter binding
        let mut sql_query = sqlx::query(&sql);

        // Bind filter parameters
        for param in &params {
            sql_query = sql_query.bind(param);
        }

        let rows = sql_query
            .fetch_all(&self.db_pool)
            .await
            .map_err(r_data_core_core::error::Error::Database)?;

        // Map rows to DynamicEntity objects
        let entities: Vec<DynamicEntity> = rows
            .iter()
            .map(|row| dynamic_entity_mapper::map_row_to_entity(row, entity_type, &entity_def))
            .collect();

        Ok(entities)
    }
}

#[async_trait]
impl DynamicEntityQueryRepositoryTrait for DynamicEntityQueryRepository {
    async fn query_entities(
        &self,
        entity_type: &str,
        query: &AdvancedEntityQuery,
    ) -> Result<Vec<DynamicEntity>> {
        Self::query_entities(self, entity_type, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_is_send_sync() {
        // Verify the repository implements Send + Sync for thread safety
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DynamicEntityQueryRepository>();
    }
}
