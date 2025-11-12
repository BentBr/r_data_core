use crate::workflow::dsl::validate_mapping;
use crate::workflow::data::adapters::auth::AuthConfig;
use crate::workflow::data::adapters::destination::HttpMethod;
use anyhow::{bail, Result};
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
        /// Mapping from normalized_field -> destination_field
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
        /// Mapping from normalized_field -> destination_field
        mapping: std::collections::HashMap<String, String>,
    },
}

pub(crate) fn validate_to(idx: usize, to: &ToDef, safe_field: &Regex) -> Result<()> {
    match to {
        ToDef::Format {
            output,
            format,
            mapping,
        } => {
            if format.format_type.trim().is_empty() {
                bail!("DSL step {}: to.format.format.format_type must not be empty", idx);
            }
            // Validate format-specific options
            match format.format_type.as_str() {
                "csv" => {
                    if let Some(delimiter) = format.options.get("delimiter").and_then(|v| v.as_str()) {
                        if delimiter.len() != 1 {
                            bail!(
                                "DSL step {}: to.format.format.options.delimiter must be a single character",
                                idx
                            );
                        }
                    }
                    if let Some(escape) = format.options.get("escape").and_then(|v| v.as_str()) {
                        if !escape.is_empty() && escape.len() != 1 {
                            bail!(
                                "DSL step {}: to.format.format.options.escape must be a single character when set",
                                idx
                            );
                        }
                    }
                    if let Some(quote) = format.options.get("quote").and_then(|v| v.as_str()) {
                        if !quote.is_empty() && quote.len() != 1 {
                            bail!(
                                "DSL step {}: to.format.format.options.quote must be a single character when set",
                                idx
                            );
                        }
                    }
                }
                "json" => {
                    // JSON format has minimal validation
                }
                _ => {
                    // Other formats will be validated by their handlers
                }
            }
            // Validate output mode
            match output {
                OutputMode::Download | OutputMode::Api => {
                    // No additional validation needed
                }
                OutputMode::Push { destination, .. } => {
                    if destination.destination_type.trim().is_empty() {
                        bail!("DSL step {}: to.format.output.push.destination.destination_type must not be empty", idx);
                    }
                    match destination.destination_type.as_str() {
                        "uri" => {
                            if let Some(uri) = destination.config.get("uri").and_then(|v| v.as_str()) {
                                if uri.trim().is_empty() {
                                    bail!("DSL step {}: to.format.output.push.destination.config.uri must not be empty", idx);
                                }
                                if !uri.starts_with("http://") && !uri.starts_with("https://") {
                                    bail!("DSL step {}: to.format.output.push.destination.config.uri must start with http:// or https://", idx);
                                }
                            } else {
                                bail!("DSL step {}: to.format.output.push.destination.config.uri is required for uri destination", idx);
                            }
                        }
                        _ => {
                            // Other destination types will be validated by their handlers
                        }
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
                bail!(
                    "DSL step {}: to.entity.entity_definition must not be empty",
                    idx
                );
            }
            if path.trim().is_empty() {
                bail!("DSL step {}: to.entity.path must not be empty", idx);
            }
            // Allow empty mappings
            validate_mapping(idx, mapping, safe_field)?;
        }
    }
    Ok(())
}

pub(crate) fn mapping_of(to: &ToDef) -> &std::collections::HashMap<String, String> {
    match to {
        ToDef::Format { mapping, .. } | ToDef::Entity { mapping, .. } => mapping,
    }
}
