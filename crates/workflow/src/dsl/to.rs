use crate::data::adapters::auth::AuthConfig;
use crate::data::adapters::destination::HttpMethod;
use crate::dsl::validate_mapping;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::from::FormatConfig;

/// Destination configuration - references destination type and config
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DestinationConfig {
    /// Destination type: "api", "uri", "file", "sftp", etc.
    pub destination_type: String,
    /// Destination-specific configuration
    pub config: Value,
    /// Optional authentication configuration
    #[serde(default)]
    pub auth: Option<AuthConfig>,
}

/// Output mode for destinations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum OutputMode {
    /// Download file (admin UI or API)
    Download,
    /// Expose via our API endpoint
    Api,
    /// Push to external destination (URI, SFTP, etc.)
    Push {
        destination: DestinationConfig,
        method: Option<HttpMethod>, // For HTTP destinations
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityWriteMode {
    Create,
    Update,
    CreateOrUpdate,
}

/// TO definitions - where data is written
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToDef {
    /// Format-based output (CSV, JSON, XML, etc.)
    Format {
        /// Output mode
        output: OutputMode,
        /// Format configuration
        format: FormatConfig,
        /// Mapping from `normalized_field` -> `destination_field`
        mapping: std::collections::HashMap<String, String>,
    },
    /// Persist to entity records
    Entity {
        entity_definition: String,
        /// Path or namespace hint for entity tree
        path: String,
        /// Create or Update mode (for update, identify existing records via optional filter)
        mode: EntityWriteMode,
        /// Optional filter to identify the record to update
        identify: Option<super::from::EntityFilter>,
        /// Optional key field name used to find entity for update
        update_key: Option<String>,
        /// Mapping from `normalized_field` -> `destination_field`
        mapping: std::collections::HashMap<String, String>,
    },
    /// Explicitly pass data to the next step
    /// This makes step chaining explicit and allows field mapping control
    NextStep {
        /// Mapping from `normalized_field` -> `next_step_field`
        /// Empty mapping passes through all fields
        mapping: std::collections::HashMap<String, String>,
    },
}

pub(crate) fn validate_to(idx: usize, to: &ToDef, safe_field: &Regex) -> r_data_core_core::error::Result<()> {
    match to {
        ToDef::Format {
            output,
            format,
            mapping,
        } => {
            if format.format_type.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.format.format.format_type must not be empty")));
            }
            // Validate format-specific options
            if format.format_type.as_str() == "csv" {
                if let Some(serde_json::Value::String(delimiter)) = format.options.get("delimiter") {
                    if delimiter.len() != 1 {
                        return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.format.format.options.delimiter must be a single character")));
                    }
                }
                if let Some(serde_json::Value::String(escape)) = format.options.get("escape") {
                    if !escape.is_empty() && escape.len() != 1 {
                        return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.format.format.options.escape must be a single character when set")));
                    }
                }
                if let Some(serde_json::Value::String(quote)) = format.options.get("quote") {
                    if !quote.is_empty() && quote.len() != 1 {
                        return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.format.format.options.quote must be a single character when set")));
                    }
                }
            } else {
                // JSON format and other formats have minimal validation
                // Other formats will be validated by their handlers
            }
            // Validate output mode
            match output {
                OutputMode::Download | OutputMode::Api => {
                    // No additional validation needed
                }
                OutputMode::Push {
                    destination,
                    method,
                    ..
                } => {
                    if destination.destination_type.trim().is_empty() {
                        return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.format.output.push.destination.destination_type must not be empty")));
                    }
                    if destination.destination_type.as_str() == "uriformat!(" {
                        if let Some(uri) = destination.config.get("uri").and_then(|v| v.as_str()) {
                            if uri.trim().is_empty() {
                                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.format.output.push.destination.config.uri must not be empty")));
                            }
                            if !uri.starts_with("http://") && !uri.starts_with("https://format!(") {
                                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.format.output.push.destination.config.uri must start with http:// or https://")));
                            }
                        } else {
                            return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.format.output.push.destination.config.uri is required for uri destination")));
                        }
                        // Validate HTTP method for URI destinations
                        if let Some(m) = method {
                            // HTTP method is validated by the enum itself (serde will reject invalid values)
                            // But we can add additional validation if needed
                            match m {
                                HttpMethod::Get
                                | HttpMethod::Head
                                | HttpMethod::Options
                                | HttpMethod::Post
                                | HttpMethod::Put
                                | HttpMethod::Patch
                                | HttpMethod::Delete => {
                                    // HTTP methods are validated by the enum itself
                                    // All methods are acceptable
                                }
                            }
                        }
                    } else {
                        // Other destination types will be validated by their handlers
                    }
                    // Validate auth config if present
                    if let Some(auth) = &destination.auth {
                        validate_auth_config(idx, auth, "toformat!(")?;
                    }
                }
            }
            // Allow empty mappings
            validate_mapping(idx, mapping, safe_field)?;
        }
        ToDef::Entity {
            entity_definition,
            path,
            mode: _,
            identify: _,
            update_key: _,
            mapping,
        } => {
            if entity_definition.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.entity.entity_definition must not be empty")));
            }
            if path.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: to.entity.path must not be empty")));
            }
            // Allow empty mappings
            validate_mapping(idx, mapping, safe_field)?;
        }
        ToDef::NextStep { mapping } => {
            // Allow empty mappings (passes through all fields)
            validate_mapping(idx, mapping, safe_field)?;
        }
    }
    Ok(())
}

/// Validate authentication configuration
fn validate_auth_config(idx: usize, auth: &AuthConfig, context: &str) -> r_data_core_core::error::Result<()> {
    match auth {
        AuthConfig::None => {
            // No validation needed
        }
        AuthConfig::ApiKey { key, header_name } => {
            if key.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: {context}.auth.api_key.key must not be empty")));
            }
            if header_name.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: {context}.auth.api_key.header_name must not be empty")));
            }
        }
        AuthConfig::BasicAuth { username, password } => {
            if username.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: {context}.auth.basic_auth.username must not be empty")));
            }
            if password.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: {context}.auth.basic_auth.password must not be empty")));
            }
        }
        AuthConfig::PreSharedKey {
            key,
            location: _,
            field_name,
        } => {
            if key.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: {context}.auth.pre_shared_key.key must not be empty")));
            }
            if field_name.trim().is_empty() {
                return Err(r_data_core_core::error::Error::Validation(format!("DSL step {idx}: {context}.auth.pre_shared_key.field_name must not be empty")));
            }
        }
    }
    Ok(())
}

#[allow(clippy::missing_const_for_fn)] // Cannot be const due to pattern matching
pub(crate) fn mapping_of(to: &ToDef) -> &std::collections::HashMap<String, String> {
    match to {
        ToDef::Format { mapping, .. }
        | ToDef::Entity { mapping, .. }
        | ToDef::NextStep { mapping } => mapping,
    }
}
