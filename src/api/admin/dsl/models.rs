use serde::{Deserialize, Serialize};
#[allow(unused_imports)] // json! macro is used in attribute macro
use serde_json::{json, Value};
use utoipa::ToSchema;

use crate::workflow::dsl::DslStep;

#[derive(Debug, Deserialize, ToSchema)]
pub struct DslValidateRequest {
    /// The DSL steps array (JSON). Example: { "steps": [ { "from": { ... }, "transform": { ... }, "to": { ... } } ] }
    #[schema(value_type = Vec<DslStep>, example = json!([
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
    pub steps: Vec<DslStep>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslValidateResponse {
    /// Whether the DSL is valid
    pub valid: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslFieldSpec {
    pub name: String,
    #[schema(example = "string")]
    pub r#type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslTypeSpec {
    pub r#type: String,
    pub fields: Vec<DslFieldSpec>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslOptionsResponse {
    pub types: Vec<DslTypeSpec>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslOptionsAndExamplesResponse {
    pub types: Vec<DslTypeSpec>,
    /// Concrete serialized examples using the real DSL structs
    pub examples: Vec<Value>,
}

