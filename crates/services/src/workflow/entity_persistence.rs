use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::value_formatting::{
    build_normalized_field_data, normalize_field_data_by_type, normalize_path,
};
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::DynamicEntity;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Context for entity persistence operations
pub struct PersistenceContext {
    pub entity_type: String,
    pub produced: Value,
    pub path: Option<String>,
    pub run_uuid: Uuid,
    pub update_key: Option<String>,
    pub skip_versioning: bool,
}

/// Result of entity lookup
pub enum EntityLookupResult {
    Found(DynamicEntity),
    NotFound,
}

/// Find an existing entity by various methods
///
/// # Errors
/// Returns an error if database query fails
#[allow(clippy::future_not_send)] // HashMap with generic BuildHasher doesn't implement Send, but this is safe in practice
pub async fn find_existing_entity<S: std::hash::BuildHasher>(
    de_service: &DynamicEntityService,
    entity_type: &str,
    normalized_field_data: &HashMap<String, Value, S>,
    original_field_data: &HashMap<String, Value, S>,
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
    ctx: &PersistenceContext,
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
    let lookup_result = find_existing_entity(
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
    let lookup_result = find_existing_entity(
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
            // Update existing entity
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

/// Resolve entity path by filters with optional value transformation
/// Resolves entity path by querying with filters
///
/// # Arguments
/// * `entity_type` - Type of entity to find
/// * `filters` - Map of field names to values (values can be transformed before lookup)
/// * `value_transforms` - Optional map of field names to transform type strings
/// * `fallback_path` - Optional path to return if entity not found (must be explicitly configured)
/// * `de_service` - Dynamic entity service
///
/// # Returns
/// `Ok(Some((path, parent_uuid)))` - Returns path if entity found or `fallback_path` is provided
/// `Ok(None)` - Returns None if entity not found and no `fallback_path` provided
///
/// # Errors
/// Returns an error if database operation fails
#[allow(clippy::implicit_hasher)] // Using default hasher (RandomState) is fine for this use case
pub async fn resolve_entity_path(
    entity_type: &str,
    filters: &HashMap<String, Value>,
    value_transforms: Option<&HashMap<String, String>>,
    fallback_path: Option<&str>,
    de_service: &DynamicEntityService,
) -> r_data_core_core::error::Result<Option<(String, Option<Uuid>)>> {
    use log::{error, warn};

    // Apply transformations to filter values
    // Note: We need to convert the HashMap types to match the function signature
    let transformed_filters = {
        let mut result = HashMap::new();
        for (k, v) in filters {
            let transformed_value = value_transforms
                .and_then(|vt| vt.get(k))
                .map_or_else(|| v.clone(), |transform_type| {
                    r_data_core_workflow::dsl::path_resolution::apply_value_transform(v, transform_type)
                });
            result.insert(k.clone(), transformed_value);
        }
        result
    };

    // Try to find entity using filter_entities
    let mut filter_map: HashMap<String, Value> =
        HashMap::new();
    for (k, v) in &transformed_filters {
        filter_map.insert(k.clone(), v.clone());
    }

    match de_service
        .filter_entities(entity_type, 1, 0, Some(filter_map), None, None, None)
        .await
    {
        Ok(entities) => {
            entities.first().map_or_else(|| fallback_path.map_or_else(
                    || {
                        warn!(
                            "Entity not found for type '{entity_type}' with filters, no fallback path configured"
                        );
                        Ok(None)
                    },
                    |fallback| {
                        warn!(
                            "Entity not found for type '{entity_type}' with filters, using configured fallback path: {fallback}"
                        );
                        Ok(Some((fallback.to_string(), None)))
                    },
                ), |entity| {
                // Extract path and parent_uuid from the entity
                let path = entity
                    .field_data
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map_or_else(|| "/".to_string(), ToString::to_string);

                let parent_uuid = entity
                    .field_data
                    .get("parent_uuid")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok());

                Ok(Some((path, parent_uuid)))
            })
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
#[allow(clippy::implicit_hasher)] // Using default hasher (RandomState) is fine for this use case
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
    let mut path_filter: HashMap<String, Value> =
        HashMap::new();
    path_filter.insert(
        "path".to_string(),
        Value::String(normalized_path.clone()),
    );
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
        let mut path_filter: HashMap<String, Value> =
            HashMap::new();
        path_filter.insert(
            "path".to_string(),
            Value::String(parent_path.clone()),
        );
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
#[allow(clippy::unused_async)]
pub async fn resolve_dynamic_path(
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
            fallback_path
                .map_or_else(|| Err(e), |fallback| Ok(fallback.to_string()))
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
#[allow(clippy::unused_async)]
#[allow(clippy::implicit_hasher)] // Using default hasher (RandomState) is fine for this use case
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
