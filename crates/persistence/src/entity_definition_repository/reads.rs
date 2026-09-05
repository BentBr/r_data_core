use super::EntityDefinitionRepository;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Error;
use r_data_core_core::error::Result;
use std::collections::HashMap;
use uuid::Uuid;

use crate::repository::PgPoolExtension;

/// List all entity definitions with pagination
///
/// # Errors
/// Returns an error if the database query fails
pub async fn list(
    repo: &EntityDefinitionRepository,
    limit: i64,
    offset: i64,
) -> Result<Vec<EntityDefinition>> {
    repo.db_pool
        .repository_with_table::<EntityDefinition>("entity_definitions")
        .list(None, Some("entity_type ASC"), Some(limit), Some(offset))
        .await
}

/// Count all entity definitions
///
/// # Errors
/// Returns an error if the database query fails
pub async fn count(repo: &EntityDefinitionRepository) -> Result<i64> {
    repo.db_pool
        .repository_with_table::<EntityDefinition>("entity_definitions")
        .count(None)
        .await
}

/// Get a entity definition by UUID
///
/// # Errors
/// Returns an error if the database query fails
pub async fn get_by_uuid(
    repo: &EntityDefinitionRepository,
    uuid: &Uuid,
) -> Result<Option<EntityDefinition>> {
    // Use custom query with explicit type casting
    let entity_def = sqlx::query!(
        r#"
        SELECT
            uuid, entity_type, display_name, description, group_name,
            allow_children, icon, field_definitions as "field_definitions: serde_json::Value",
            created_at, updated_at,
            created_by as "created_by: Uuid", updated_by,
            published, version
        FROM entity_definitions
        WHERE uuid = $1
        "#,
        uuid
    )
    .fetch_optional(&repo.db_pool)
    .await
    .map_err(Error::Database)?;

    if let Some(entity_def) = entity_def {
        // Create schema
        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(entity_def.entity_type.clone()),
        );
        let schema = r_data_core_core::entity_definition::schema::Schema::new(properties);

        // Convert to EntityDefinition
        let definition = EntityDefinition {
            uuid: entity_def.uuid,
            entity_type: entity_def.entity_type,
            display_name: entity_def.display_name,
            description: entity_def.description,
            group_name: entity_def.group_name,
            allow_children: entity_def.allow_children,
            icon: entity_def.icon,
            fields: serde_json::from_value(entity_def.field_definitions)
                .map_err(Error::Serialization)?,
            schema,
            created_at: entity_def.created_at,
            updated_at: entity_def.updated_at,
            created_by: entity_def.created_by,
            updated_by: entity_def.updated_by,
            published: entity_def.published,
            version: entity_def.version,
        };
        Ok(Some(definition))
    } else {
        Ok(None)
    }
}

/// Get a entity definition by entity type
///
/// # Errors
/// Returns an error if the database query fails
pub async fn get_by_entity_type(
    repo: &EntityDefinitionRepository,
    entity_type: &str,
) -> Result<Option<EntityDefinition>> {
    let entity_def = sqlx::query!(
        r#"
        SELECT
            uuid, entity_type, display_name, description, group_name,
            allow_children, icon, field_definitions as "field_definitions: serde_json::Value",
            created_at, updated_at,
            created_by as "created_by: Uuid", updated_by,
            published, version
        FROM entity_definitions
        WHERE entity_type = $1
        "#,
        entity_type
    )
    .fetch_optional(&repo.db_pool)
    .await
    .map_err(Error::Database)?;

    if let Some(entity_def) = entity_def {
        // Create schema
        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(entity_def.entity_type.clone()),
        );
        let schema = r_data_core_core::entity_definition::schema::Schema::new(properties);

        // Convert to EntityDefinition
        Ok(Some(EntityDefinition {
            uuid: entity_def.uuid,
            entity_type: entity_def.entity_type,
            display_name: entity_def.display_name,
            description: entity_def.description,
            group_name: entity_def.group_name,
            allow_children: entity_def.allow_children,
            icon: entity_def.icon,
            fields: serde_json::from_value(entity_def.field_definitions)
                .map_err(Error::Serialization)?,
            schema,
            created_at: entity_def.created_at,
            updated_at: entity_def.updated_at,
            created_by: entity_def.created_by,
            updated_by: entity_def.updated_by,
            published: entity_def.published,
            version: entity_def.version,
        }))
    } else {
        Ok(None)
    }
}
