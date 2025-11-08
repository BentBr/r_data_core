use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DslStepKind {
    Map,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DslStep {
    #[serde(rename = "type")]
    pub kind: DslStepKind,
    pub from: String,
    pub to: String,
    /// Optional expression applied to the source value. Currently ignored (stub).
    pub expr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DslProgram {
    pub steps: Vec<DslStep>,
}

impl DslProgram {
    pub fn from_config(config: &Value) -> anyhow::Result<Self> {
        let steps = config
            .get("dsl")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let parsed: Vec<DslStep> = steps
            .into_iter()
            .map(|v| serde_json::from_value::<DslStep>(v).context("Invalid DSL step"))
            .collect::<Result<_, _>>()?;
        Ok(DslProgram { steps: parsed })
    }

    /// Apply DSL to a raw input payload to produce an entity-like object.
    /// Current implementation supports only simple `map` steps and ignores `expr`.
    pub fn apply(&self, input: &Value) -> anyhow::Result<Value> {
        let mut out = json!({});
        for step in &self.steps {
            match step.kind {
                DslStepKind::Map => {
                    let source_value = get_source_value(input, &step.from);
                    // For now, ignore expression; pass-through value
                    set_target_value(&mut out, &step.to, source_value.unwrap_or(Value::Null));
                }
            }
        }
        Ok(out)
    }
}

fn get_source_value(input: &Value, from: &str) -> Option<Value> {
    // Accept forms like "csv.price", "ndjson.amount", "payload.field", "field"
    let key = from.split('.').last().unwrap_or(from);
    match input {
        Value::Object(map) => map.get(key).cloned(),
        _ => None,
    }
}

fn set_target_value(target: &mut Value, to: &str, val: Value) {
    // Accept forms like "entity.amount" -> set under "amount" on the root object for now.
    // Later we can support nested paths.
    let key = to.split('.').last().unwrap_or(to);
    if let Value::Object(obj) = target {
        obj.insert(key.to_string(), val);
    }
}


