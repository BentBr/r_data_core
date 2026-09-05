use super::EntityDefinitionRepository;
use crate::entity_definition_versioning_repository::EntityDefinitionVersioningRepository;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::entity_definition::repository_trait::EntityDefinitionRepositoryTrait;
use r_data_core_core::error::Error;
use r_data_core_core::error::Result;
use r_data_core_core::field::types::FieldType;
use uuid::Uuid;

/// Create a new entity definition
///
/// # Errors
/// Returns an error if the database operation fails
pub async fn create(
    repo: &EntityDefinitionRepository,
    definition: &EntityDefinition,
) -> Result<Uuid> {
    // We need a custom implementation because the general repository requires a path field
    // that entity definitions don't have

    // Build the SQL fields and values
    let entity_type = &definition.entity_type;
    let display_name = &definition.display_name;
    let description = definition.description.as_ref();
    let group_name = definition.group_name.as_ref();
    let allow_children = definition.allow_children;
    let icon = definition.icon.as_ref();
    let fields = serde_json::to_value(&definition.fields).map_err(Error::Serialization)?;
    let created_at = definition.created_at;
    let updated_at = definition.updated_at;
    let created_by: Uuid = definition.created_by;
    let updated_by = definition.updated_by;
    let published = definition.published;
    let version = definition.version;

    // Log values for debugging
    log::debug!("Creating entity definition");
    log::debug!("Entity type: {entity_type}");
    log::debug!(
        "Created by: {created_by} (type: {})",
        std::any::type_name_of_val(&created_by)
    );
    log::debug!("Fields: {fields}");
    log::debug!("Schema properties: {:?}", definition.schema.properties);

    // SQL query with named parameters for clarity
    let query = "INSERT INTO entity_definitions
                (entity_type, display_name, description, group_name, allow_children,
                 icon, field_definitions, created_at, updated_at, created_by, updated_by,
                 published, version)
                VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                RETURNING uuid";

    let result = sqlx::query_scalar::<_, Uuid>(query)
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
        .fetch_one(&repo.db_pool)
        .await
        .map_err(|e| {
            log::error!("Database error creating entity definition: {e}");
            Error::Database(e)
        })?;

    // Explicitly create or update the entity table and view is already done by the trigger
    // No need to call it here as it will be handled by the database

    Ok(result)
}

/// Update an existing entity definition
///
/// # Errors
/// Returns an error if the database operation fails
pub async fn update(
    repo: &EntityDefinitionRepository,
    uuid: &Uuid,
    definition: &EntityDefinition,
) -> Result<()> {
    // Custom implementation for entity definitions that doesn't require a path field

    // Build the SQL fields and values
    let entity_type = &definition.entity_type;
    let display_name = &definition.display_name;
    let description = definition.description.as_ref();
    let group_name = definition.group_name.as_ref();
    let allow_children = definition.allow_children;
    let icon = definition.icon.as_ref();
    let fields = serde_json::to_value(&definition.fields).map_err(Error::Serialization)?;
    let updated_at = definition.updated_at;
    let updated_by = definition.updated_by;
    let published = definition.published;

    // Start a transaction
    let mut tx = repo.db_pool.begin().await?;

    // Pre-update snapshot of current definition (within transaction)
    EntityDefinitionVersioningRepository::snapshot_pre_update_tx(
        &mut tx,
        *uuid,
        definition.updated_by,
    )
    .await?;

    // SQL query - increment version atomically in SQL (like entities and workflows)
    let query = "UPDATE entity_definitions SET
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
                version = version + 1
                WHERE uuid = $11";

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
        .bind(uuid)
        .execute(&mut *tx)
        .await
        .map_err(Error::Database)?;

    // Commit the transaction
    tx.commit().await?;

    Ok(())
}

/// Delete a entity definition
///
/// # Errors
/// Returns an error if the database operation fails
pub async fn delete(repo: &EntityDefinitionRepository, uuid: &Uuid) -> Result<()> {
    // First, get the entity definition to get the entity type
    let entity_definition_result = repo.get_by_uuid(uuid).await?;

    if let Some(entity_definition) = entity_definition_result {
        let table_name = entity_definition.get_table_name();

        // Drop the entity table if it exists
        let table_exists = repo.check_view_exists(&table_name).await?;
        if table_exists {
            // Also drop any relation tables if they exist
            for field in &entity_definition.fields {
                if field.field_type == FieldType::ManyToMany {
                    let relation_table_name = format!(
                        "rel_{}_{}",
                        entity_definition.entity_type.to_lowercase(),
                        field.name.to_lowercase()
                    );

                    let rel_table_exists = repo.check_view_exists(&relation_table_name).await?;
                    if rel_table_exists {
                        log::info!("Dropping relation table: {relation_table_name}");
                        let drop_rel_sql =
                            format!("DROP TABLE IF EXISTS {relation_table_name} CASCADE");
                        sqlx::query(&drop_rel_sql)
                            .execute(&repo.db_pool)
                            .await
                            .map_err(Error::Database)?;
                    }
                }
            }

            // Drop the entity table
            log::info!("Dropping entity table: {table_name}");
            let drop_entity_sql = format!("DROP TABLE IF EXISTS {table_name} CASCADE");
            sqlx::query(&drop_entity_sql)
                .execute(&repo.db_pool)
                .await
                .map_err(Error::Database)?;
        }

        // Remove the entity from the custom entities registry
        repo.delete_from_entities_registry(&entity_definition.entity_type)
            .await?;

        // Finally, delete the definition from the entity_definitions table
        sqlx::query("DELETE FROM entity_definitions WHERE uuid = $1")
            .bind(uuid)
            .execute(&repo.db_pool)
            .await
            .map_err(Error::Database)?;

        Ok(())
    } else {
        Err(r_data_core_core::error::Error::NotFound(format!(
            "Class definition with UUID {uuid} not found"
        )))
    }
}
