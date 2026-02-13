#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::collections::HashMap;

use r_data_core_core::error::Result;
use r_data_core_workflow::dsl::{build_path_from_fields, transform::Transform, StringOperand};
use serde_json::Value;

/// JWT configuration for authenticate transforms
pub struct JwtConfig<'a> {
    /// Base JWT secret (entity tokens use `{secret}_entity` suffix)
    pub secret: Option<&'a str>,
    /// Default token expiration in seconds
    pub expiration: u64,
}

// Helper functions to access nested values (re-implementing execution module functions)
fn get_nested(input: &Value, path: &str) -> Option<Value> {
    let mut current = input;
    for key in path.split('.') {
        match current {
            Value::Object(map) => {
                if let Some(v) = map.get(key) {
                    current = v;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(current.clone())
}

fn set_nested(target: &mut Value, path: &str, val: Value) {
    let mut acc = val;
    for key in path.split('.').rev() {
        let mut map = serde_json::Map::new();
        map.insert(key.to_string(), acc);
        acc = Value::Object(map);
    }
    merge_objects(target, &acc);
}

fn merge_objects(target: &mut Value, addition: &Value) {
    match (target, addition) {
        (Value::Object(tobj), Value::Object(aobj)) => {
            for (k, v) in aobj {
                if let Some(existing) = tobj.get_mut(k) {
                    merge_objects(existing, v);
                } else {
                    tobj.insert(k.clone(), v.clone());
                }
            }
        }
        (t, v) => {
            *t = v.clone();
        }
    }
}

use crate::dynamic_entity::DynamicEntityService;
use crate::workflow::entity_persistence::{get_or_create_entity_by_path, resolve_entity_path};
use uuid::Uuid;

/// Execute async transforms that require database access
///
/// # Arguments
/// * `transform` - Transform to execute
/// * `normalized` - Normalized data context
/// * `de_service` - Dynamic entity service
/// * `run_uuid` - Workflow run UUID
/// * `jwt` - JWT configuration for authenticate transforms
///
/// # Returns
/// Modified normalized data with transform results
///
/// # Errors
/// Returns an error if transform execution fails
pub async fn execute_async_transform(
    transform: &Transform,
    normalized: &mut Value,
    de_service: &DynamicEntityService,
    run_uuid: Uuid,
    jwt: &JwtConfig<'_>,
) -> Result<()> {
    match transform {
        Transform::ResolveEntityPath(rep) => {
            handle_resolve_entity_path(rep, normalized, de_service).await
        }
        Transform::GetOrCreateEntity(goc) => {
            handle_get_or_create_entity(goc, normalized, de_service, run_uuid).await
        }
        Transform::Authenticate(auth) => {
            handle_authenticate(auth, normalized, de_service, jwt).await
        }
        _ => {
            // Other transforms are handled synchronously in DSL execution
            Ok(())
        }
    }
}

async fn handle_resolve_entity_path(
    rep: &r_data_core_workflow::dsl::transform::ResolveEntityPathTransform,
    normalized: &mut Value,
    de_service: &DynamicEntityService,
) -> Result<()> {
    // Evaluate filter operands to get filter values
    let filters = evaluate_filter_operands(&rep.filters, normalized)?;

    // Resolve entity path
    let result = resolve_entity_path(
        &rep.entity_type,
        &filters,
        rep.value_transforms.as_ref(),
        rep.fallback_path.as_deref(),
        de_service,
    )
    .await?;

    match result {
        Some((path, entity_uuid)) => {
            // Set the resolved path
            set_nested(normalized, &rep.target_path, Value::String(path));

            // Set the entity's UUID (use as parent_uuid for children)
            if let Some(ref target_uuid) = rep.target_uuid {
                if let Some(uuid) = entity_uuid {
                    set_nested(normalized, target_uuid, Value::String(uuid.to_string()));
                }
            }
        }
        None => {
            // Entity not found and no fallback path configured - fail the workflow
            return Err(r_data_core_core::error::Error::Validation(format!(
                "Entity of type '{}' not found with given filters and no fallback path configured. Use 'get_or_create_entity' transform or configure a fallback_path.",
                rep.entity_type
            )));
        }
    }
    Ok(())
}

async fn handle_get_or_create_entity(
    goc: &r_data_core_workflow::dsl::transform::GetOrCreateEntityTransform,
    normalized: &mut Value,
    de_service: &DynamicEntityService,
    run_uuid: Uuid,
) -> Result<()> {
    // First, build the path from template
    let path = build_path_from_fields::<std::collections::hash_map::RandomState>(
        &goc.path_template,
        normalized,
        goc.path_separator.as_deref(),
        None, // Field transforms would be applied in build_path_from_fields if needed
    )?;

    // Prepare field data for creation if needed
    let create_field_data = prepare_create_field_data(goc.create_field_data.as_ref(), normalized)?;

    // Get or create entity (returns path, parent_uuid, entity_uuid)
    let (path_result, _parent_uuid, entity_uuid) = get_or_create_entity_by_path(
        &goc.entity_type,
        &path,
        create_field_data,
        de_service,
        run_uuid,
    )
    .await?;

    // Set results in normalized data
    set_nested(normalized, &goc.target_path, Value::String(path_result));

    // Set the entity's UUID (use as parent_uuid for children)
    if let Some(target_uuid) = &goc.target_uuid {
        set_nested(
            normalized,
            target_uuid,
            Value::String(entity_uuid.to_string()),
        );
    }
    Ok(())
}

fn evaluate_filter_operands(
    filters: &std::collections::HashMap<String, StringOperand>,
    normalized: &Value,
) -> Result<std::collections::HashMap<String, Value>> {
    let mut result = std::collections::HashMap::new();
    for (field, operand) in filters {
        let filter_value = match operand {
            StringOperand::Field { field: field_path } => get_nested(normalized, field_path)
                .ok_or_else(|| {
                    r_data_core_core::error::Error::Validation(format!(
                        "Field '{field_path}' not found for filter '{field}'"
                    ))
                })?,
            StringOperand::ConstString { value } => Value::String(value.clone()),
        };
        result.insert(field.clone(), filter_value);
    }
    Ok(result)
}

fn prepare_create_field_data(
    create_data: Option<&std::collections::HashMap<String, StringOperand>>,
    normalized: &Value,
) -> Result<Option<std::collections::HashMap<String, Value>>> {
    create_data.map_or(Ok(None), |create_data| {
        let mut field_data = std::collections::HashMap::new();
        for (field, operand) in create_data {
            let field_value = match operand {
                StringOperand::Field { field: field_path } => get_nested(normalized, field_path)
                    .ok_or_else(|| {
                        r_data_core_core::error::Error::Validation(format!(
                            "Field '{field_path}' not found for create_field_data '{field}'"
                        ))
                    })?,
                StringOperand::ConstString { value } => Value::String(value.clone()),
            };
            field_data.insert(field.clone(), field_value);
        }
        Ok(Some(field_data))
    })
}

async fn handle_authenticate(
    auth: &r_data_core_workflow::dsl::transform::AuthenticateTransform,
    normalized: &mut Value,
    de_service: &DynamicEntityService,
    jwt: &JwtConfig<'_>,
) -> Result<()> {
    let base_secret = jwt.secret.ok_or_else(|| {
        r_data_core_core::error::Error::Config(
            "JWT secret not configured — cannot issue entity tokens".to_string(),
        )
    })?;

    // 1. Extract identifier and password from normalized data
    let identifier = get_nested(normalized, &auth.input_identifier)
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| r_data_core_core::error::Error::Auth("Invalid credentials".to_string()))?;
    let password = get_nested(normalized, &auth.input_password)
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| r_data_core_core::error::Error::Auth("Invalid credentials".to_string()))?;

    // 2. Look up the entity by the identifier field
    let mut filters = HashMap::new();
    filters.insert(auth.identifier_field.clone(), Value::String(identifier));
    let entity = de_service
        .find_one_by_filters(&auth.entity_type, &filters)
        .await?
        .ok_or_else(|| r_data_core_core::error::Error::Auth("Invalid credentials".to_string()))?;

    // 3. Read the raw password hash (bypasses redaction)
    let entity_uuid = entity
        .field_data
        .get("uuid")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or_else(|| r_data_core_core::error::Error::Auth("Invalid credentials".to_string()))?;

    let password_hash = de_service
        .get_raw_field_value(&auth.entity_type, &entity_uuid, &auth.password_field)
        .await?
        .ok_or_else(|| r_data_core_core::error::Error::Auth("Invalid credentials".to_string()))?;

    // 4. Verify the password
    if !r_data_core_core::crypto::verify_password_argon2(&password, &password_hash) {
        return Err(r_data_core_core::error::Error::Auth(
            "Invalid credentials".to_string(),
        ));
    }

    // 5. Collect extra claims from entity data
    let mut extra = HashMap::new();
    for (claim_name, entity_field) in &auth.extra_claims {
        if let Some(val) = entity.field_data.get(entity_field) {
            extra.insert(claim_name.clone(), val.clone());
        }
    }

    // 6. Generate entity JWT
    let expiry = auth.token_expiry_seconds.unwrap_or(jwt.expiration);
    let token = r_data_core_core::entity_jwt::generate_entity_jwt(
        &entity_uuid.to_string(),
        &auth.entity_type,
        extra,
        base_secret,
        expiry,
    )?;

    // 7. Set the token in normalized data
    set_nested(normalized, &auth.target_token, Value::String(token));

    Ok(())
}
