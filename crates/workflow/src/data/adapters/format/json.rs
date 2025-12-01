use super::FormatHandler;
use anyhow::Result;
use bytes::Bytes;
use serde_json::Value;

/// JSON format handler
#[derive(Default)]
pub struct JsonFormatHandler;

impl JsonFormatHandler {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl FormatHandler for JsonFormatHandler {
    fn format_type(&self) -> &'static str {
        "json"
    }

    /// # Errors
    /// Returns an error if JSON parsing fails.
    fn parse(&self, data: &[u8], _options: &Value) -> Result<Vec<Value>> {
        // Try parsing as array first
        if let Ok(array) = serde_json::from_slice::<Vec<Value>>(data) {
            return Ok(array);
        }

        // Try parsing as NDJSON (newline-delimited)
        let mut results = Vec::new();
        for line in data.split(|&b| b == b'\n') {
            if line.is_empty() {
                continue;
            }
            match serde_json::from_slice::<Value>(line) {
                Ok(value) => results.push(value),
                Err(e) => {
                    // If it's not valid JSON, try the whole thing as a single object
                    if results.is_empty() {
                        return Err(e.into());
                    }
                }
            }
        }

        if !results.is_empty() {
            return Ok(results);
        }

        // Try parsing as single object
        let obj = serde_json::from_slice::<Value>(data)?;
        Ok(vec![obj])
    }

    /// # Errors
    /// Returns an error if JSON serialization fails.
    fn serialize(&self, data: &[Value], options: &Value) -> Result<Bytes> {
        let as_array = options
            .get("as_array")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let bytes = if as_array {
            serde_json::to_vec(&data)?
        } else {
            // NDJSON format
            let mut buf = Vec::new();
            for value in data {
                buf.extend_from_slice(&serde_json::to_vec(value)?);
                buf.push(b'\n');
            }
            buf
        };

        Ok(Bytes::from(bytes))
    }

    /// # Errors
    /// Returns an error if the configuration is invalid.
    fn validate_options(&self, _options: &Value) -> Result<()> {
        // JSON format has minimal options
        Ok(())
    }
}
