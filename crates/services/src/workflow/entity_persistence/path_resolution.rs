#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
#![allow(clippy::implicit_hasher)]

use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::value_formatting::normalize_path;
use r_data_core_core::DynamicEntity;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Result of entity path resolution: (path, `entity_uuid`)
///
/// - `path`: The entity's path
/// - `entity_uuid`: The entity's UUID (use as `parent_uuid` for children)
pub type ResolvedEntityPath = (String, Option<Uuid>);

/// Look up an entity by its full path and return (path, uuid)
///
/// Full path is the combination of parent path + `entity_key`. For example,
/// an entity with `path="/Clients"` and `entity_key="unlicensed"` has full path
/// `/Clients/unlicensed`.
///
/// # Errors
/// Returns an error if the entity is not found at the given path
async fn lookup_entity_by_path(
    entity_type: &str,
    full_path: &str,
    de_service: &DynamicEntityService,
) -> r_data_core_core::error::Result<Option<ResolvedEntityPath>> {
    use log::error;
    use r_data_core_workflow::dsl::path_resolution::parse_entity_path;

    let normalized_full_path = normalize_path(full_path);

    // Parse the full path into parent_path and entity_key
    // For "/Clients/unlicensed", this gives parent_path="/Clients", entity_key="unlicensed"
    let (_, entity_key, parent_path_opt) = parse_entity_path(&normalized_full_path);

    if entity_key.is_empty() {
        error!("Cannot lookup entity at root path '/'");
        return Err(r_data_core_core::error::Error::NotFound(
            "Cannot lookup entity at root path '/'".to_string(),
        ));
    }

    // Build filter: match on parent path AND entity_key
    let mut filter_map: HashMap<String, Value> = HashMap::new();
    filter_map.insert("entity_key".to_string(), Value::String(entity_key.clone()));

    // For entities at root level (e.g., "/mykey"), parent_path is None, meaning path = "/"
    let parent_path = parent_path_opt.unwrap_or_else(|| "/".to_string());
    filter_map.insert("path".to_string(), Value::String(parent_path));

    match de_service
        .filter_entities(entity_type, 1, 0, Some(filter_map), None, None, None)
        .await
    {
        Ok(entities) => {
            if let Some(entity) = entities.first() {
                let entity_uuid = entity
                    .field_data
                    .get("uuid")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok());

                Ok(Some((normalized_full_path, entity_uuid)))
            } else {
                error!(
                    "Fallback entity not found at path '{normalized_full_path}' for type '{entity_type}'. Please create the fallback entity first."
                );
                Err(r_data_core_core::error::Error::NotFound(format!(
                    "Fallback entity not found at path '{normalized_full_path}' for type '{entity_type}'. Please create the fallback entity first."
                )))
            }
        }
        Err(e) => {
            error!("Failed to lookup entity by path '{full_path}': {e}");
            Err(e)
        }
    }
}

/// Resolve entity path by filters with optional value transformation
///
/// # Arguments
/// * `entity_type` - Type of entity to find
/// * `filters` - Map of field names to values (values can be transformed before lookup)
/// * `value_transforms` - Optional map of field names to transform type strings
/// * `fallback_path` - Optional path to return if entity not found (must be explicitly configured)
/// * `de_service` - Dynamic entity service
///
/// # Returns
/// `Ok(Some((path, entity_uuid)))` - Returns path and UUID if entity found or fallback
/// `Ok(None)` - Returns None if entity not found and no `fallback_path` provided
///
/// # Errors
/// Returns an error if the database operation fails
pub async fn resolve_entity_path(
    entity_type: &str,
    filters: &HashMap<String, Value>,
    value_transforms: Option<&HashMap<String, String>>,
    fallback_path: Option<&str>,
    de_service: &DynamicEntityService,
) -> r_data_core_core::error::Result<Option<ResolvedEntityPath>> {
    use log::{error, warn};

    // Apply transformations to filter values
    let transformed_filters = {
        let mut result = HashMap::new();
        for (k, v) in filters {
            let transformed_value = value_transforms.and_then(|vt| vt.get(k)).map_or_else(
                || v.clone(),
                |transform_type| {
                    r_data_core_workflow::dsl::path_resolution::apply_value_transform(
                        v,
                        transform_type,
                    )
                },
            );
            result.insert(k.clone(), transformed_value);
        }
        result
    };

    // Try to find entity using filter_entities
    let mut filter_map: HashMap<String, Value> = HashMap::new();
    for (k, v) in &transformed_filters {
        filter_map.insert(k.clone(), v.clone());
    }

    let entities = de_service
        .filter_entities(entity_type, 1, 0, Some(filter_map), None, None, None)
        .await;

    match entities {
        Ok(entities) => {
            if let Some(entity) = entities.first() {
                // Entity found - construct full path from parent_path + entity_key
                // The `path` field stores the parent path, not the full path.
                // Full path = path + '/' + entity_key (or '/' + entity_key for root entities)
                let parent_path = entity
                    .field_data
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("/");

                let entity_key = entity
                    .field_data
                    .get("entity_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let full_path = if parent_path == "/" {
                    format!("/{entity_key}")
                } else {
                    format!("{parent_path}/{entity_key}")
                };

                let entity_uuid = entity
                    .field_data
                    .get("uuid")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok());

                Ok(Some((full_path, entity_uuid)))
            } else {
                // Entity not found - try fallback
                match fallback_path {
                    None => {
                        warn!(
                            "Entity not found for type '{entity_type}' with filters, no fallback path configured"
                        );
                        Ok(None)
                    }
                    Some(fallback) => {
                        warn!(
                            "Entity not found for type '{entity_type}' with filters, using fallback: {fallback}"
                        );
                        // Look up the fallback entity by path to get its UUID
                        lookup_entity_by_path(entity_type, fallback, de_service).await
                    }
                }
            }
        }
        Err(e) => {
            // Database error - return error, don't use fallback
            error!("Database error while resolving entity path for {entity_type}: {e}");
            Err(e)
        }
    }
}

/// Get or create entity by path
///
/// # Arguments
/// * `entity_type` - Type of entity to get/create
/// * `path` - Full path to the entity
/// * `field_data` - Field data for entity creation if needed
/// * `de_service` - Dynamic entity service
/// * `run_uuid` - Workflow run UUID for audit fields
///
/// # Returns
/// `Ok((path, parent_uuid, entity_uuid))` - Always returns entity (created or found)
///
/// # Errors
/// Returns an error if the database operation fails
pub async fn get_or_create_entity_by_path(
    entity_type: &str,
    path: &str,
    field_data: Option<HashMap<String, Value>>,
    de_service: &DynamicEntityService,
    run_uuid: Uuid,
) -> r_data_core_core::error::Result<(String, Option<Uuid>, Uuid)> {
    use r_data_core_workflow::dsl::path_resolution::parse_entity_path;

    let (normalized_path, entity_key, parent_path_opt): (String, String, Option<String>) =
        parse_entity_path(path);

    if entity_key.is_empty() {
        return Err(r_data_core_core::error::Error::Validation(
            "Cannot create entity at root path".to_string(),
        ));
    }

    // Try to find entity by path using filter with path
    let mut path_filter: HashMap<String, Value> = HashMap::new();
    path_filter.insert("path".to_string(), Value::String(normalized_path.clone()));
    let entities = de_service
        .filter_entities(entity_type, 1, 0, Some(path_filter), None, None, None)
        .await?;

    if let Some(entity) = entities.first() {
        // Entity exists, return its info
        let entity_uuid = entity
            .field_data
            .get("uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| {
                r_data_core_core::error::Error::Entity("Entity found but missing UUID".to_string())
            })?;

        let parent_uuid = entity
            .field_data
            .get("parent_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());

        return Ok((normalized_path, parent_uuid, entity_uuid));
    }

    // Entity doesn't exist, create it
    // Determine parent_uuid if parent path exists
    let parent_uuid = if let Some(parent_path) = parent_path_opt {
        let mut path_filter: HashMap<String, Value> = HashMap::new();
        path_filter.insert("path".to_string(), Value::String(parent_path.clone()));
        let parent_entities = de_service
            .filter_entities(entity_type, 1, 0, Some(path_filter), None, None, None)
            .await?;
        parent_entities.first().and_then(|e| {
            e.field_data
                .get("uuid")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
        })
    } else {
        None
    };

    // Build field data for the new entity
    let mut new_field_data = field_data.unwrap_or_default();
    new_field_data.insert("entity_key".to_string(), Value::String(entity_key));
    new_field_data.insert("path".to_string(), Value::String(normalized_path.clone()));
    if let Some(parent_uuid_val) = parent_uuid {
        new_field_data.insert(
            "parent_uuid".to_string(),
            Value::String(parent_uuid_val.to_string()),
        );
    }
    new_field_data.insert(
        "created_by".to_string(),
        Value::String(run_uuid.to_string()),
    );
    new_field_data.insert(
        "updated_by".to_string(),
        Value::String(run_uuid.to_string()),
    );

    // Create the entity using the service
    let entity = DynamicEntity {
        entity_type: entity_type.to_string(),
        field_data: new_field_data,
        definition: Arc::new(
            de_service
                .entity_definition_service()
                .get_entity_definition_by_entity_type(entity_type)
                .await?,
        ),
    };

    let entity_uuid = de_service.create_entity(&entity).await?;

    Ok((normalized_path, parent_uuid, entity_uuid))
}

/// Resolve dynamic path from entity lookup or path building
///
/// # Arguments
/// * `path_template` - Path template (e.g., `/statistics_instance/{license_key_id}/statistics_submission`)
/// * `context` - Workflow context with field data
/// * `de_service` - Dynamic entity service for lookups
/// * `fallback_path` - Optional fallback path if resolution fails
///
/// # Returns
/// Resolved path or fallback
///
/// # Errors
/// Returns an error if path building fails
pub fn resolve_dynamic_path(
    path_template: &str,
    context: &Value,
    _de_service: &DynamicEntityService,
    fallback_path: Option<&str>,
) -> r_data_core_core::error::Result<String> {
    use r_data_core_workflow::dsl::path_resolution::build_path_from_fields;

    // Build path from template
    match build_path_from_fields::<std::collections::hash_map::RandomState>(
        path_template,
        context,
        None,
        None,
    ) {
        Ok(path) => Ok(path),
        Err(e) => {
            // If path building fails, use fallback
            fallback_path.map_or_else(|| Err(e), |fallback| Ok(fallback.to_string()))
        }
    }
}

/// Get or create parent entity by path and return its path and UUID
///
/// # Arguments
/// * `entity_type` - Type of parent entity
/// * `path` - Full path to parent entity
/// * `field_data` - Optional field data for creation
/// * `de_service` - Dynamic entity service
/// * `run_uuid` - Workflow run UUID
///
/// # Returns
/// `(path, parent_uuid)` of the parent entity
///
/// # Errors
/// Returns an error if the database operation fails
pub async fn get_or_create_parent_entity(
    entity_type: &str,
    path: &str,
    field_data: Option<HashMap<String, Value>>,
    de_service: &DynamicEntityService,
    run_uuid: Uuid,
) -> r_data_core_core::error::Result<(String, Option<Uuid>)> {
    let (path_result, _parent_uuid, entity_uuid) =
        get_or_create_entity_by_path(entity_type, path, field_data, de_service, run_uuid).await?;

    // For parent entity, we return the path and its UUID (which becomes parent_uuid for children)
    Ok((path_result, Some(entity_uuid)))
}
