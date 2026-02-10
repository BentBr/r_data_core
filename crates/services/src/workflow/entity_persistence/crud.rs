#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::implicit_hasher)]

use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::value_formatting::normalize_path;
use r_data_core_core::DynamicEntity;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::lookup::{
    build_final_field_data, ensure_audit_fields, ensure_entity_key, prepare_field_data,
};
use super::{EntityLookupResult, PersistenceContext};

/// Derive and enforce path from `parent_uuid` by looking up parent entity
///
/// **IMPORTANT**: When `parent_uuid` is set, this function ALWAYS derives the path from the parent
/// to ensure data integrity. The path is calculated as:
/// `{parent_path}/{parent_entity_key}/{child_entity_key}`
///
/// This ensures that:
/// 1. The entity's path is always consistent with its parent relationship
/// 2. The path reflects the actual hierarchy in the entity tree
///
/// If `parent_uuid` is NOT set, the function requires that a path is already present.
///
/// # Errors
/// Returns an error if:
/// - `parent_uuid` is set but parent entity lookup fails
/// - Neither `parent_uuid` nor `path` is provided
async fn derive_path_from_parent<S: std::hash::BuildHasher>(
    de_service: &DynamicEntityService,
    field_data: &mut HashMap<String, Value, S>,
) -> r_data_core_core::error::Result<()> {
    // Check if parent_uuid is set
    let Some(parent_uuid_str) = field_data
        .get("parent_uuid")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
    else {
        // No parent_uuid - path must be provided
        if field_data.contains_key("path") {
            return Ok(());
        }
        return Err(r_data_core_core::error::Error::Validation(
            "Either 'path' or 'parent_uuid' must be provided for entity creation".to_string(),
        ));
    };

    // parent_uuid is set - ALWAYS derive path from parent for consistency
    // This ensures path is always: parent_path + "/" + parent_entity_key + "/" + child_entity_key

    // Parse parent UUID
    let parent_uuid = Uuid::parse_str(&parent_uuid_str).map_err(|e| {
        r_data_core_core::error::Error::Validation(format!("Invalid parent_uuid: {e}"))
    })?;

    // Look up the parent entity (search across all entity types)
    let parent = de_service
        .get_entity_by_uuid_any_type(parent_uuid)
        .await
        .map_err(|e| {
            r_data_core_core::error::Error::NotFound(format!(
                "Parent entity with UUID {parent_uuid} not found: {e}"
            ))
        })?;

    // Get parent's path (this is the parent's parent path)
    let parent_path = parent
        .field_data
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            r_data_core_core::error::Error::Validation(format!(
                "Parent entity {parent_uuid} has no path"
            ))
        })?;

    // Get parent's entity_key
    let parent_entity_key = parent
        .field_data
        .get("entity_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            r_data_core_core::error::Error::Validation(format!(
                "Parent entity {parent_uuid} has no entity_key"
            ))
        })?;

    // Calculate the parent's full path: parent_path + "/" + parent_entity_key
    let parent_full_path = if parent_path == "/" {
        format!("/{parent_entity_key}")
    } else {
        format!("{parent_path}/{parent_entity_key}")
    };

    // Validate that child entity_key exists (must already be set at this point)
    // We don't use the value here - it's used elsewhere to construct the full path
    let _entity_key = field_data
        .get("entity_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            r_data_core_core::error::Error::Validation(
                "entity_key must be set before deriving path from parent_uuid".to_string(),
            )
        })?;

    // Derive child's path as parent_full_path (which becomes the child's parent path)
    // The child's full path would be: parent_full_path + "/" + entity_key
    // But we store the parent path in the 'path' field, so: path = parent_full_path
    let normalized_path = normalize_path(&parent_full_path);
    field_data.insert("path".to_string(), Value::String(normalized_path));

    Ok(())
}

/// Create a new entity
///
/// # Errors
/// Returns an error if entity definition not found, validation fails, or database operation fails
pub async fn create_entity(
    de_service: &DynamicEntityService,
    ctx: &PersistenceContext,
) -> r_data_core_core::error::Result<()> {
    let (field_data, def) = prepare_field_data(de_service, ctx).await?;

    let normalized_field_data = build_final_field_data(field_data, &def);

    let mut final_data = normalized_field_data;

    // Ensure audit fields exist
    ensure_audit_fields(&mut final_data, ctx.run_uuid);

    // Ensure entity_key exists
    ensure_entity_key(de_service, &ctx.entity_type, &mut final_data).await;

    // Derive path from parent_uuid if path is not set
    derive_path_from_parent(de_service, &mut final_data).await?;

    // Log missing required fields for debugging
    let missing_required_fields: Vec<String> = def
        .fields
        .iter()
        .filter(|f| f.required && !final_data.contains_key(&f.name))
        .map(|f| f.name.clone())
        .collect();
    if !missing_required_fields.is_empty() {
        log::warn!(
            "persist_entity_create: Missing required fields for entity_type={}, run_uuid={}, missing_fields={:?}, produced_keys={:?}",
            ctx.entity_type,
            ctx.run_uuid,
            missing_required_fields,
            ctx.produced.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()
        );
    }

    let entity = DynamicEntity {
        entity_type: ctx.entity_type.clone(),
        field_data: final_data,
        definition: Arc::new(def),
    };
    let _uuid = de_service.create_entity(&entity).await?;
    Ok(())
}

/// Update an existing entity
///
/// # Errors
/// Returns an error if entity not found, validation fails, or database operation fails
pub async fn update_entity(
    de_service: &DynamicEntityService,
    ctx: &PersistenceContext,
) -> r_data_core_core::error::Result<()> {
    let (field_data, def) = prepare_field_data(de_service, ctx).await?;
    let original_field_data = field_data.clone();
    let normalized_field_data = build_final_field_data(field_data, &def);

    // Find existing entity
    let lookup_result = super::find_existing_entity(
        de_service,
        &ctx.entity_type,
        &normalized_field_data,
        &original_field_data,
        &ctx.produced,
        ctx.update_key.as_deref(),
    )
    .await?;

    let mut entity = match lookup_result {
        EntityLookupResult::Found(e) => e,
        EntityLookupResult::NotFound => {
            return Err(r_data_core_core::error::Error::NotFound(
                "Entity not found for update. Provide uuid or entity_key in the data.".to_string(),
            ));
        }
    };

    // Update the entity's field_data with new values
    for (k, v) in &normalized_field_data {
        // Don't overwrite created_at or created_by
        if k != "created_at" && k != "created_by" {
            entity.field_data.insert(k.clone(), v.clone());
        }
    }

    // Set updated_by to run_uuid
    entity.field_data.insert(
        "updated_by".to_string(),
        Value::String(ctx.run_uuid.to_string()),
    );

    // Ensure uuid is set (should already be present from existing entity)
    if !entity.field_data.contains_key("uuid") {
        return Err(r_data_core_core::error::Error::Entity(
            "Cannot update entity: missing uuid".to_string(),
        ));
    }

    de_service
        .update_entity_with_options(&entity, ctx.skip_versioning)
        .await?;
    Ok(())
}

/// Create or update an entity (upsert)
///
/// # Errors
/// Returns an error if entity definition not found, validation fails, or database operation fails
pub async fn create_or_update_entity(
    de_service: &DynamicEntityService,
    ctx: &PersistenceContext,
) -> r_data_core_core::error::Result<()> {
    let (field_data, def) = prepare_field_data(de_service, ctx).await?;
    let original_field_data = field_data.clone();
    let normalized_field_data = build_final_field_data(field_data, &def);

    // Try to find existing entity
    let lookup_result = super::find_existing_entity(
        de_service,
        &ctx.entity_type,
        &normalized_field_data,
        &original_field_data,
        &ctx.produced,
        ctx.update_key.as_deref(),
    )
    .await?;

    match lookup_result {
        EntityLookupResult::Found(mut entity) => {
            // Update the existing entity
            for (k, v) in &normalized_field_data {
                // Don't overwrite created_at or created_by
                if k != "created_at" && k != "created_by" {
                    entity.field_data.insert(k.clone(), v.clone());
                }
            }

            // Set updated_by to run_uuid
            entity.field_data.insert(
                "updated_by".to_string(),
                Value::String(ctx.run_uuid.to_string()),
            );

            de_service
                .update_entity_with_options(&entity, ctx.skip_versioning)
                .await?;
        }
        EntityLookupResult::NotFound => {
            // Create new entity
            let mut final_data = normalized_field_data;

            // Ensure audit fields exist
            ensure_audit_fields(&mut final_data, ctx.run_uuid);

            // Ensure entity_key exists
            ensure_entity_key(de_service, &ctx.entity_type, &mut final_data).await;

            // Derive path from parent_uuid if parent_uuid is set
            // This ensures path is always consistent with parent relationship
            derive_path_from_parent(de_service, &mut final_data).await?;

            let entity = DynamicEntity {
                entity_type: ctx.entity_type.clone(),
                field_data: final_data,
                definition: Arc::new(def),
            };
            let _uuid = de_service.create_entity(&entity).await?;
        }
    }
    Ok(())
}
