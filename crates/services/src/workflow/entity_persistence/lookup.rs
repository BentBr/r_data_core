#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::implicit_hasher)]

use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::value_formatting::{
    build_normalized_field_data, normalize_field_data_by_type, normalize_path,
};
use r_data_core_core::entity_definition::definition::EntityDefinition;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use super::EntityLookupResult;

/// Find an existing entity by various methods
///
/// # Errors
/// Returns an error if database query fails
pub async fn find_existing_entity(
    de_service: &DynamicEntityService,
    entity_type: &str,
    normalized_field_data: &HashMap<String, Value>,
    original_field_data: &HashMap<String, Value>,
    produced: &Value,
    update_key: Option<&str>,
) -> r_data_core_core::error::Result<EntityLookupResult> {
    // First, try to find by UUID if present
    if let Some(Value::String(uuid_str)) = normalized_field_data.get("uuid") {
        if let Ok(uuid) = Uuid::parse_str(uuid_str) {
            if let Ok(Some(entity)) = de_service
                .get_entity_by_uuid(entity_type, &uuid, None)
                .await
            {
                return Ok(EntityLookupResult::Found(entity));
            }
        }
    }

    // If not found by UUID, try to find by update_key or entity_key
    let search_key = update_key
        .and_then(|key_field| {
            // Use the update_key field name to find the value in produced data
            normalized_field_data
                .get(key_field)
                .or_else(|| original_field_data.get(key_field))
                .or_else(|| produced.as_object().and_then(|obj| obj.get(key_field)))
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
        .or_else(|| {
            normalized_field_data
                .get("entity_key")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
        .or_else(|| {
            original_field_data
                .get("entity_key")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        })
        .or_else(|| {
            produced
                .get("entity_key")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        });

    if let Some(key_value) = search_key {
        // Use filter_entities to find by entity_key
        let mut filters = HashMap::new();
        filters.insert("entity_key".to_string(), Value::String(key_value));
        if let Ok(entities) = de_service
            .filter_entities(entity_type, 1, 0, Some(filters), None, None, None)
            .await
        {
            if let Some(entity) = entities.first() {
                return Ok(EntityLookupResult::Found(entity.clone()));
            }
        }
    }

    Ok(EntityLookupResult::NotFound)
}

/// Prepare field data for persistence (normalize path, types, etc.)
///
/// # Errors
/// Returns an error if entity definition not found or database query fails
pub async fn prepare_field_data(
    de_service: &DynamicEntityService,
    ctx: &super::PersistenceContext,
) -> r_data_core_core::error::Result<(HashMap<String, Value>, EntityDefinition)> {
    // Build field_data as a flat object from produced
    let mut field_data = HashMap::new();
    if let Some(obj) = ctx.produced.as_object() {
        for (k, v) in obj {
            field_data.insert(k.clone(), v.clone());
        }
    }

    // Normalize path
    if let Some(p) = &ctx.path {
        let normalized_path = normalize_path(p);
        field_data.insert("path".to_string(), Value::String(normalized_path));
    } else if let Some(path_value) = field_data.get("path") {
        if let Some(path_str) = path_value.as_str() {
            let normalized_path = normalize_path(path_str);
            field_data.insert("path".to_string(), Value::String(normalized_path));
        }
    }

    // Fetch entity definition
    let defs = de_service.entity_definition_service();
    let def = defs
        .get_entity_definition_by_entity_type(&ctx.entity_type)
        .await?;

    // Normalize field data types based on entity definition
    normalize_field_data_by_type(&mut field_data, &def);

    Ok((field_data, def))
}

/// Build the final normalized field data with reserved field handling
pub fn build_final_field_data<S: std::hash::BuildHasher + Default>(
    field_data: HashMap<String, Value, S>,
    entity_definition: &EntityDefinition,
) -> HashMap<String, Value> {
    build_normalized_field_data(field_data, entity_definition)
}

/// Ensure required audit fields exist for entity creation
pub fn ensure_audit_fields<S: std::hash::BuildHasher>(
    field_data: &mut HashMap<String, Value, S>,
    run_uuid: Uuid,
) {
    field_data
        .entry("created_by".to_string())
        .or_insert_with(|| Value::String(run_uuid.to_string()));
    field_data
        .entry("updated_by".to_string())
        .or_insert_with(|| Value::String(run_uuid.to_string()));
}

/// Generate `entity_key` if missing
pub async fn ensure_entity_key<S: std::hash::BuildHasher>(
    de_service: &DynamicEntityService,
    entity_type: &str,
    field_data: &mut HashMap<String, Value, S>,
) {
    if !field_data.contains_key("entity_key") {
        let existing_count: i64 = de_service.count_entities(entity_type).await.unwrap_or(0);
        let rand = Uuid::now_v7().to_string();
        let short = &rand[..8];
        let key = format!("{}-{}-{}", entity_type, existing_count + 1, short);
        field_data.insert("entity_key".to_string(), Value::String(key));
    }
}
