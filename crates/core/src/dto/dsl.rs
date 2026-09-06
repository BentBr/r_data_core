#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]
//! DSL API DTOs.
//!
//! `Deserialize` is derived on the response types as well as the request
//! types: API clients need to parse what the server sends. It has no effect on
//! the generated TypeScript.

use serde::{Deserialize, Serialize};
#[allow(unused_imports)] // json! macro is used in attribute macro
use serde_json::{json, Value};
use ts_rs::TS;
use utoipa::ToSchema;

// Note: DslStep is imported from the main crate's workflow module
// This is a temporary dependency until workflow is migrated to a crate
// For now, we use serde_json::Value to represent the steps
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DslValidateRequest {
    /// The DSL steps array (JSON). Example: { "steps": [ { "from": { ... }, "transform": { ... }, "to": { ... } } ] }
    #[schema(value_type = Vec<Value>, example = json!([
        {
            "from": {
                "type": "format",
                "source": {
                    "source_type": "uri",
                    "config": { "uri": "http://example.com/data.csv" },
                    "auth": { "type": "none" }
                },
                "format": {
                    "format_type": "csv",
                    "options": { "has_header": true, "delimiter": "," }
                },
                "mapping": { "price": "price" }
            },
            "transform": {
                "type": "arithmetic",
                "target": "price",
                "left": { "kind": "field", "field": "price" },
                "op": "add",
                "right": { "kind": "const", "value": 5.0 }
            },
            "to": {
                "type": "format",
                "output": { "mode": "api" },
                "format": {
                    "format_type": "json",
                    "options": {}
                },
                "mapping": { "price": "entity.total" }
            }
        }
    ]))]
    #[ts(type = "unknown[]")]
    pub steps: Vec<Value>, // Will be Vec<DslStep> once workflow is migrated
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DslValidateResponse {
    /// Whether the DSL is valid
    pub valid: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DslFieldSpec {
    pub name: String,
    #[schema(example = "string")]
    pub r#type: String,
    pub required: bool,
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DslTypeSpec {
    pub r#type: String,
    pub fields: Vec<DslFieldSpec>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DslOptionsResponse {
    pub types: Vec<DslTypeSpec>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DslOptionsAndExamplesResponse {
    pub types: Vec<DslTypeSpec>,
    /// Concrete serialized examples using the real DSL structs
    #[ts(type = "unknown[]")]
    pub examples: Vec<Value>,
}

/// What a step actually did during a dry-run.
///
/// The distinction matters to a caller deciding whether to trust the result:
/// only `Executed` proves a step works. `Suppressed` means the step was
/// described but never attempted, because the effect could not be undone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StepEffect {
    /// Ran for real against an in-memory overlay of the database. Nothing was
    /// persisted, but the step is proven to work: mappings resolved, paths
    /// built, lookups returned what they will return in production.
    Executed,
    /// Not attempted. Sending an email or making an outbound request cannot be
    /// undone, so the dry-run describes it instead of performing it.
    Suppressed,
    /// Nothing persistent was involved either way.
    NoSideEffect,
}

/// One step of a dry-run.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DryRunStepTrace {
    #[ts(type = "number")]
    pub step_index: usize,
    /// What the destination received, or would have.
    #[ts(type = "unknown")]
    pub produced: Value,
    /// The step's `to` definition, echoed so the caller can see the target.
    #[ts(type = "unknown")]
    pub to: Value,
    pub effect: StepEffect,
    /// For a suppressed step, what would have happened.
    pub suppressed_detail: Option<String>,
}

/// Request body for `POST /admin/api/v1/dsl/dry-run`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DryRunRequest {
    /// An unsaved DSL program, so a draft can be tested before it exists.
    #[ts(type = "unknown[]")]
    pub steps: Vec<Value>,
    /// Sample input the first step reads.
    #[ts(type = "unknown")]
    pub input: Value,
}

/// Result of a dry-run.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DryRunResponse {
    pub steps: Vec<DryRunStepTrace>,
    /// Entities the run would have created or updated, as
    /// `"create customer 0195..."`. Empty when the program writes nothing.
    pub would_write: Vec<String>,
}
