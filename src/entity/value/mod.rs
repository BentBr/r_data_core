mod conversion;

pub use conversion::*;

use std::collections::HashMap;
use serde_json::Value as JsonValue;
use crate::error::{Error, Result};

// Implement ToValue for serde_json::Value directly
impl ToValue for serde_json::Value {
    fn to_value(&self) -> Result<JsonValue> {
        // Since we're converting from JsonValue to JsonValue, just clone it
        Ok(self.clone())
    }
}