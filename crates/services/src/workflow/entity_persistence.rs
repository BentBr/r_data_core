use r_data_core_core::DynamicEntity;
use r_data_core_core::entity_definition::definition::EntityDefinition;
use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::value_formatting::{
    build_normalized_field_data, normalize_field_data_by_type, normalize_path,
};
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
pub async fn find_existing_entity(
    de_service: &DynamicEntityService,
    entity_type: &str,
    normalized_field_data: &HashMap<String, Value>,
    original_field_data: &HashMap<String, Value>,
    produced: &Value,
    update_key: Option<&String>,
) -> anyhow::Result<EntityLookupResult> {
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
    let search_key = if let Some(key_field) = update_key {
        // Use the update_key field name to find the value in produced data
        normalized_field_data
            .get(key_field)
            .or_else(|| original_field_data.get(key_field))
            .or_else(|| {
                if let Some(produced_obj) = produced.as_object() {
                    produced_obj.get(key_field)
                } else {
                    None
                }
            })
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else if let Some(Value::String(key)) = normalized_field_data.get("entity_key") {
        Some(key.clone())
    } else if let Some(Value::String(key)) = original_field_data.get("entity_key") {
        Some(key.clone())
    } else if let Some(Value::String(key)) = produced.get("entity_key") {
        Some(key.clone())
    } else {
        None
    };

    if let Some(key_value) = search_key {
        // Use filter_entities to find by entity_key
        let mut filters = HashMap::new();
        filters.insert("entity_key".to_string(), Value::String(key_value));
        if let Ok(entities) = de_service
            .filter_entities(entity_type, 1, 0, Some(filters), None, None, None)
            .await
        {
            if let Some(entity) = entities.first() {
                return Ok(EntityLookupResult::Found(entity.clone() as DynamicEntity));
            }
        }
    }

    Ok(EntityLookupResult::NotFound)
}

/// Prepare field data for persistence (normalize path, types, etc.)
pub async fn prepare_field_data(
    de_service: &DynamicEntityService,
    ctx: &PersistenceContext,
) -> anyhow::Result<(HashMap<String, Value>, EntityDefinition)> {
    // Build field_data as a flat object from produced
    let mut field_data = HashMap::new();
    if let Some(obj) = ctx.produced.as_object() {
        for (k, v) in obj.iter() {
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
pub fn build_final_field_data(
    field_data: HashMap<String, Value>,
    _entity_definition: &EntityDefinition,
) -> HashMap<String, Value> {
    build_normalized_field_data(field_data, _entity_definition)
}

/// Ensure required audit fields exist for entity creation
pub fn ensure_audit_fields(field_data: &mut HashMap<String, Value>, run_uuid: Uuid) {
    field_data
        .entry("created_by".to_string())
        .or_insert_with(|| Value::String(run_uuid.to_string()));
    field_data
        .entry("updated_by".to_string())
        .or_insert_with(|| Value::String(run_uuid.to_string()));
}

/// Generate entity_key if missing
pub async fn ensure_entity_key(
    de_service: &DynamicEntityService,
    entity_type: &str,
    field_data: &mut HashMap<String, Value>,
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
pub async fn create_entity(
    de_service: &DynamicEntityService,
    ctx: &PersistenceContext,
) -> anyhow::Result<()> {
    let (field_data, def) = prepare_field_data(de_service, ctx).await?;

    let normalized_field_data = build_final_field_data(field_data, &def);

    // Force uuid generation (repository requires uuid on create)
    let mut final_data = normalized_field_data;
    final_data
        .entry("uuid".to_string())
        .or_insert_with(|| Value::String(Uuid::now_v7().to_string()));

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
    de_service.create_entity(&entity).await?;
    Ok(())
}

/// Update an existing entity
pub async fn update_entity(
    de_service: &DynamicEntityService,
    ctx: &PersistenceContext,
) -> anyhow::Result<()> {
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
        ctx.update_key.as_ref(),
    )
    .await?;

    let mut entity = match lookup_result {
        EntityLookupResult::Found(e) => e,
        EntityLookupResult::NotFound => {
            return Err(anyhow::anyhow!(
                "Entity not found for update. Provide uuid or entity_key in the data."
            ));
        }
    };

    // Update the entity's field_data with new values
    for (k, v) in normalized_field_data.iter() {
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
        return Err(anyhow::anyhow!("Cannot update entity: missing uuid"));
    }

    de_service
        .update_entity_with_options(&entity, ctx.skip_versioning)
        .await?;
    Ok(())
}

/// Create or update an entity (upsert)
pub async fn create_or_update_entity(
    de_service: &DynamicEntityService,
    ctx: &PersistenceContext,
) -> anyhow::Result<()> {
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
        ctx.update_key.as_ref(),
    )
    .await?;

    match lookup_result {
        EntityLookupResult::Found(mut entity) => {
            // Update existing entity
            for (k, v) in normalized_field_data.iter() {
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

            // Force uuid generation
            final_data
                .entry("uuid".to_string())
                .or_insert_with(|| Value::String(Uuid::now_v7().to_string()));

            // Ensure audit fields exist
            ensure_audit_fields(&mut final_data, ctx.run_uuid);

            // Ensure entity_key exists
            ensure_entity_key(de_service, &ctx.entity_type, &mut final_data).await;

            let entity = DynamicEntity {
                entity_type: ctx.entity_type.clone(),
                field_data: final_data,
                definition: Arc::new(def),
            };
            de_service.create_entity(&entity).await?;
        }
    }
    Ok(())
}
