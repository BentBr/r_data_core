use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transform {
    Arithmetic(ArithmeticTransform),
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
