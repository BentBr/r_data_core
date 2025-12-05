#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use r_data_core_core::public_api::{BrowseNode, EntityTypeInfo};
use sqlx::PgPool;

mod browse;
mod utils;

pub use browse::browse_by_path;
pub use utils::get_entity_count;

/// Repository for public API operations on dynamic entities
///
/// This repository provides read-only access to dynamic entities and their entity definitions
/// through the public API. Dynamic entities are instances defined by entity definitions.
pub struct DynamicEntityPublicRepository {
    db_pool: PgPool,
}

impl DynamicEntityPublicRepository {
    /// Create a new dynamic entity public repository
    #[must_use]
    pub const fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// List all available entity types (entity definitions) with metadata
    ///
    /// Returns entity definitions that are published and can be used to create dynamic entities.
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn list_available_entity_types(&self) -> Result<Vec<EntityTypeInfo>> {
        let rows = sqlx::query!(
            "
            SELECT entity_type as name, display_name, description,
                   uuid as entity_definition_uuid
            FROM entity_definitions
            WHERE published = true
            ",
        )
        .fetch_all(&self.db_pool)
        .await?;

        let mut result = Vec::new();

        for row in rows {
            // Count entity instances
            let entity_count = get_entity_count(&self.db_pool, &row.name)
                .await
                .unwrap_or(0);

            // Count fields for each entity
            let field_count: i64 = sqlx::query_scalar!(
                "
                SELECT COUNT(*) as count
                FROM entity_definitions
                WHERE entity_type = $1
                ",
                row.name
            )
            .fetch_one(&self.db_pool)
            .await
            .map_or(0, |count| count.unwrap_or(0));

            result.push(EntityTypeInfo {
                name: row.name,
                display_name: row.display_name,
                description: row.description,
                is_system: false,
                entity_count,
                #[allow(clippy::cast_possible_truncation)]
                field_count: field_count as i32,
            });
        }

        Ok(result)
    }

    /// Browse dynamic entities by virtual path
    ///
    /// Returns folders and files representing the hierarchical structure of dynamic entities.
    ///
    /// # Errors
    /// Returns an error if the database query fails
    pub async fn browse_by_path(
        &self,
        raw_path: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<BrowseNode>, i64)> {
        browse::browse_by_path(&self.db_pool, raw_path, limit, offset).await
    }
}
