use serde_json::Value;

use super::DslProgram;

pub struct DslProcessor {
    program: DslProgram,
}

impl DslProcessor {
    pub fn new(program: DslProgram) -> Self {
        Self { program }
    }

    pub fn process_item(&self, payload: &Value) -> anyhow::Result<Value> {
        self.program.apply(payload)
    }
}
