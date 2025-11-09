use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transform {
    Arithmetic(ArithmeticTransform),
    /// No-op transform (optional step)
    None,
    /// Concatenate two string operands (optionally with a separator) into target
    Concat(ConcatTransform),
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
/// Future: ExternalEntityField for cross-entity lookups during transform.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Operand {
    Field {
        field: String,
    },
    Const {
        value: f64,
    },
    /// Future extension: resolve from repository during processing (not implemented in apply())
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

pub(crate) fn validate_transform(idx: usize, t: &Transform, safe_field: &Regex) -> Result<()> {
    match t {
        Transform::Arithmetic(ar) => {
            if !safe_field.is_match(&ar.target) {
                bail!(
                    "DSL step {}: transform.arithmetic.target must be a safe identifier",
                    idx
                );
            }
            validate_operand(idx, "left", &ar.left, safe_field)?;
            validate_operand(idx, "right", &ar.right, safe_field)?;
        }
        Transform::Concat(ct) => {
            if !safe_field.is_match(&ct.target) {
                bail!(
                    "DSL step {}: transform.concat.target must be a safe identifier",
                    idx
                );
            }
            match &ct.left {
                StringOperand::Field { field } => {
                    if !safe_field.is_match(field) {
                        bail!(
                            "DSL step {}: transform.concat.left field path must be safe",
                            idx
                        );
                    }
                }
                StringOperand::ConstString { .. } => {}
            }
            match &ct.right {
                StringOperand::Field { field } => {
                    if !safe_field.is_match(field) {
                        bail!(
                            "DSL step {}: transform.concat.right field path must be safe",
                            idx
                        );
                    }
                }
                StringOperand::ConstString { .. } => {}
            }
        }
        Transform::None => {}
    }
    Ok(())
}

fn validate_operand(idx: usize, side: &str, op: &Operand, safe_field: &Regex) -> Result<()> {
    match op {
        Operand::Field { field } => {
            if !safe_field.is_match(field) {
                bail!(
                    "DSL step {}: transform.arithmetic.{} field path must be safe",
                    idx,
                    side
                );
            }
        }
        Operand::Const { .. } => {}
        Operand::ExternalEntityField {
            entity_definition,
            filter,
            field,
        } => {
            if entity_definition.trim().is_empty() {
                bail!(
                    "DSL step {}: transform.arithmetic.{} external entity_definition required",
                    idx,
                    side
                );
            }
            if filter.field.trim().is_empty() || filter.value.trim().is_empty() {
                bail!(
                    "DSL step {}: transform.arithmetic.{} external filter requires field and value",
                    idx,
                    side
                );
            }
            if !safe_field.is_match(field) {
                bail!(
                    "DSL step {}: transform.arithmetic.{} external field path must be safe",
                    idx,
                    side
                );
            }
        }
    }
    Ok(())
}
