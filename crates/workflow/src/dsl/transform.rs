use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transform {
    Arithmetic(ArithmeticTransform),
    /// No-op transform (optional step)
    None,
    /// Concatenate two string operands (optionally with a separator) into the target
    Concat(ConcatTransform),
    /// Resolve entity path by filters with optional value transformation
    ResolveEntityPath(ResolveEntityPathTransform),
    /// Build the path from input fields with placeholders
    BuildPath(BuildPathTransform),
    /// Get or create the entity by path
    GetOrCreateEntity(GetOrCreateEntityTransform),
    /// Authenticate credentials against entity data and issue an entity JWT
    Authenticate(AuthenticateTransform),
}

/// Arithmetic transform allows setting a target field to the result of left (op) right.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArithmeticTransform {
    /// Target normalized field to set
    pub target: String,
    pub left: Operand,
    pub op: ArithmeticOp,
    pub right: Operand,
}

/// String concatenation transform
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConcatTransform {
    /// Target normalized field to set
    pub target: String,
    pub left: StringOperand,
    /// Optional separator between left and right
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    pub right: StringOperand,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Operand can reference a normalized field or be a constant value.
/// Future: `ExternalEntityField` for cross-entity lookups during transform.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Operand {
    Field {
        field: String,
    },
    Const {
        value: f64,
    },
    /// Future extension: resolve from repository during processing (not implemented in `apply()`)
    ExternalEntityField {
        entity_definition: String,
        filter: super::from::EntityFilter,
        field: String,
    },
}

/// String operand variant used by Concat transform
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StringOperand {
    Field { field: String },
    ConstString { value: String },
}

/// Resolve entity path transform - finds entity by filters and sets path and UUID
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResolveEntityPathTransform {
    /// Target field to store the resolved path
    pub target_path: String,
    /// Optional target field to store the found entity's UUID (use as `parent_uuid` for children)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_uuid: Option<String>,
    /// Entity type to query
    pub entity_type: String,
    /// Filters to find the entity (field -> value mapping)
    pub filters: std::collections::HashMap<String, StringOperand>,
    /// Optional value transformations to apply before lookup
    /// Map of field name -> transform type (e.g., "lowercase", "trim", "normalize")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_transforms: Option<std::collections::HashMap<String, String>>,
    /// Fallback path if entity not found (configurable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_path: Option<String>,
}

/// Build path transform - builds path from template with placeholders
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BuildPathTransform {
    /// Target field to store the built path
    pub target: String,
    /// Path template with placeholders (e.g., "/{field1}/{field2}")
    pub template: String,
    /// Optional separator between segments (default: "/")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    /// Optional transforms to apply to field values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_transforms: Option<std::collections::HashMap<String, String>>,
}

/// Get or create entity transform - gets entity by path or creates it
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetOrCreateEntityTransform {
    /// Target field to store the entity path
    pub target_path: String,
    /// Optional target field to store the entity's UUID (use as `parent_uuid` for children)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_uuid: Option<String>,
    /// Entity type to get/create
    pub entity_type: String,
    /// Path template to build (e.g., "/{field1}/{field2}")
    pub path_template: String,
    /// Optional field data to use when creating entity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_field_data: Option<std::collections::HashMap<String, StringOperand>>,
    /// Optional separator for path building (default: "/")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_separator: Option<String>,
}

/// Authenticate transform — verifies credentials against entity data and issues a JWT.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticateTransform {
    /// Entity type holding user records (e.g. "user")
    pub entity_type: String,
    /// Entity field to match the submitted identifier against (e.g. "email")
    pub identifier_field: String,
    /// Entity field containing the password hash
    pub password_field: String,
    /// Normalized input field with the submitted identifier
    pub input_identifier: String,
    /// Normalized input field with the submitted password
    pub input_password: String,
    /// Output field name for the issued JWT token
    pub target_token: String,
    /// Extra JWT claims: claim name → entity field name
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra_claims: HashMap<String, String>,
    /// Override the default token expiry (seconds). Falls back to `JWT_EXPIRATION` env.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_expiry_seconds: Option<u64>,
}

pub(crate) fn validate_transform(
    idx: usize,
    t: &Transform,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    match t {
        Transform::Arithmetic(ar) => validate_arithmetic_transform(idx, ar, safe_field)?,
        Transform::Concat(ct) => validate_concat_transform(idx, ct, safe_field)?,
        Transform::ResolveEntityPath(rep) => {
            validate_resolve_entity_path_transform(idx, rep, safe_field)?;
        }
        Transform::BuildPath(bp) => validate_build_path_transform(idx, bp, safe_field)?,
        Transform::GetOrCreateEntity(goc) => {
            validate_get_or_create_entity_transform(idx, goc, safe_field)?;
        }
        Transform::Authenticate(auth) => {
            validate_authenticate_transform(idx, auth, safe_field)?;
        }
        Transform::None => {}
    }
    Ok(())
}

fn validate_arithmetic_transform(
    idx: usize,
    ar: &ArithmeticTransform,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    if !safe_field.is_match(&ar.target) {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.arithmetic.target must be a safe identifier"
        )));
    }
    validate_operand(idx, "left", &ar.left, safe_field)?;
    validate_operand(idx, "right", &ar.right, safe_field)?;
    Ok(())
}

fn validate_concat_transform(
    idx: usize,
    ct: &ConcatTransform,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    if !safe_field.is_match(&ct.target) {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.concat.target must be a safe identifier"
        )));
    }
    validate_string_operand(idx, "left", &ct.left, safe_field)?;
    validate_string_operand(idx, "right", &ct.right, safe_field)?;
    Ok(())
}

fn validate_string_operand(
    idx: usize,
    side: &str,
    operand: &StringOperand,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    if let StringOperand::Field { field } = operand {
        if !safe_field.is_match(field) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "DSL step {idx}: transform.concat.{side} field path must be safe"
            )));
        }
    }
    Ok(())
}

fn validate_resolve_entity_path_transform(
    idx: usize,
    rep: &ResolveEntityPathTransform,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    if !safe_field.is_match(&rep.target_path) {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.resolve_entity_path.target_path must be a safe identifier"
        )));
    }
    if let Some(ref target_uuid) = rep.target_uuid {
        if !safe_field.is_match(target_uuid) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "DSL step {idx}: transform.resolve_entity_path.target_uuid must be a safe identifier"
            )));
        }
    }
    if rep.entity_type.trim().is_empty() {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.resolve_entity_path.entity_type must not be empty"
        )));
    }
    if rep.filters.is_empty() {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.resolve_entity_path.filters must not be empty"
        )));
    }
    for (field, operand) in &rep.filters {
        if let StringOperand::Field { field: field_path } = operand {
            if !safe_field.is_match(field_path) {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "DSL step {idx}: transform.resolve_entity_path.filters.{field} field path must be safe"
                )));
            }
        }
    }
    Ok(())
}

fn validate_build_path_transform(
    idx: usize,
    bp: &BuildPathTransform,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    if !safe_field.is_match(&bp.target) {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.build_path.target must be a safe identifier"
        )));
    }
    if bp.template.trim().is_empty() {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.build_path.template must not be empty"
        )));
    }
    Ok(())
}

fn validate_get_or_create_entity_transform(
    idx: usize,
    goc: &GetOrCreateEntityTransform,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    if !safe_field.is_match(&goc.target_path) {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.get_or_create_entity.target_path must be a safe identifier"
        )));
    }
    if let Some(ref target_uuid) = goc.target_uuid {
        if !safe_field.is_match(target_uuid) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "DSL step {idx}: transform.get_or_create_entity.target_uuid must be a safe identifier"
            )));
        }
    }
    if goc.entity_type.trim().is_empty() {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.get_or_create_entity.entity_type must not be empty"
        )));
    }
    if goc.path_template.trim().is_empty() {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.get_or_create_entity.path_template must not be empty"
        )));
    }
    if let Some(ref create_data) = goc.create_field_data {
        for (field, operand) in create_data {
            if let StringOperand::Field { field: field_path } = operand {
                if !safe_field.is_match(field_path) {
                    return Err(r_data_core_core::error::Error::Validation(format!(
                        "DSL step {idx}: transform.get_or_create_entity.create_field_data.{field} field path must be safe"
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_authenticate_transform(
    idx: usize,
    auth: &AuthenticateTransform,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    if auth.entity_type.trim().is_empty() {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.authenticate.entity_type must not be empty"
        )));
    }
    if !safe_field.is_match(&auth.entity_type) {
        return Err(r_data_core_core::error::Error::Validation(format!(
            "DSL step {idx}: transform.authenticate.entity_type must be a safe identifier"
        )));
    }
    for (label, value) in [
        ("identifier_field", &auth.identifier_field),
        ("password_field", &auth.password_field),
        ("input_identifier", &auth.input_identifier),
        ("input_password", &auth.input_password),
        ("target_token", &auth.target_token),
    ] {
        if !safe_field.is_match(value) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "DSL step {idx}: transform.authenticate.{label} must be a safe identifier"
            )));
        }
    }
    for (claim_name, entity_field) in &auth.extra_claims {
        if claim_name.trim().is_empty() {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "DSL step {idx}: transform.authenticate.extra_claims key must not be empty"
            )));
        }
        if !safe_field.is_match(entity_field) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "DSL step {idx}: transform.authenticate.extra_claims.{claim_name} field must be a safe identifier"
            )));
        }
    }
    Ok(())
}

fn validate_operand(
    idx: usize,
    side: &str,
    op: &Operand,
    safe_field: &Regex,
) -> r_data_core_core::error::Result<()> {
    match op {
        Operand::Field { field } => {
            if !safe_field.is_match(field) {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "DSL step {idx}: transform.arithmetic.{side} field path must be safe"
                )));
            }
        }
        Operand::Const { .. } => {}
        Operand::ExternalEntityField {
            entity_definition,
            filter,
            field,
        } => {
            if entity_definition.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: transform.arithmetic.{side} external entity_definition required")));
            }
            if filter.field.trim().is_empty() || filter.value.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: transform.arithmetic.{side} external filter requires field and value")));
            }
            if !safe_field.is_match(field) {
                return Err(r_data_core_core::error::Error::Validation(format!(
                    "DSL step {idx}: transform.arithmetic.{side} external field path must be safe"
                )));
            }
        }
    }
    Ok(())
}
