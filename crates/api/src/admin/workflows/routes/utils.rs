#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde_json::Value;

/// Check if a workflow config has from.api source type (accepts POST, cron disabled)
/// or to.format.output.mode === 'api' (exports via GET, cron disabled)
pub(crate) fn check_has_api_endpoint(config: &Value) -> bool {
    if let Some(steps) = config.get("steps").and_then(|v| v.as_array()) {
        for step in steps {
            // Check for from.api source type (accepts POST, cron disabled)
            if let Some(from) = step.get("from") {
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

            // Check for to.format.output.mode === 'api' (exports via GET, cron disabled)
            if let Some(to) = step.get("to") {
                if let Some(to_type) = to.get("type").and_then(|v| v.as_str()) {
                    if to_type == "format" {
                        if let Some(output) = to.get("output") {
                            // Check if output is a string "api" or object with mode: "api"
                            if output.as_str() == Some("api") {
                                return true;
                            }
                            if let Some(output_obj) = output.as_object() {
                                if output_obj.get("mode").and_then(|v| v.as_str()) == Some("api") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
