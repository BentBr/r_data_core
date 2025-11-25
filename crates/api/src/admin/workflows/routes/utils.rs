#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::Value;

/// Check if a workflow config has from.api source type (accepts POST, cron disabled)
pub(crate) fn check_has_api_endpoint(config: &Value) -> bool {
    if let Some(steps) = config.get("steps").and_then(|v| v.as_array()) {
        for step in steps {
            if let Some(from) = step.get("from") {
                // Check for from.format.source.source_type === "api" without endpoint field
                if let Some(source) = from
                    .get("source")
                    .or_else(|| from.get("format").and_then(|f| f.get("source")))
                {
                    if let Some(source_type) = source.get("source_type").and_then(|v| v.as_str()) {
                        if source_type == "api" {
                            // from.api without endpoint field = accepts POST
                            if let Some(config_obj) =
                                source.get("config").and_then(|v| v.as_object())
                            {
                                if !config_obj.contains_key("endpoint") {
                                    return true;
                                }
                            } else {
                                // No config object or empty config = accepts POST
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

