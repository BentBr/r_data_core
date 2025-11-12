use crate::workflow::dsl::validate_mapping;
use crate::workflow::data::adapters::auth::{AuthConfig, KeyLocation};
use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityFilter {
    pub field: String,
    pub value: String,
}

/// Source configuration - references source type and config
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceConfig {
    /// Source type: "uri", "file", "api", "sftp", etc.
    pub source_type: String,
    /// Source-specific configuration
    pub config: Value,
    /// Optional authentication configuration
    #[serde(default)]
    pub auth: Option<AuthConfig>,
}

/// Format configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormatConfig {
    /// Format type: "csv", "json", "xml", etc.
    pub format_type: String,
    /// Format-specific options
    #[serde(default)]
    pub options: Value,
}

/// FROM definitions - where data originates
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromDef {
    /// Format-based input (CSV, JSON, XML, etc.)
    Format {
        /// Source configuration (URI, File, API, etc.)
        source: SourceConfig,
        /// Format configuration
        format: FormatConfig,
        /// Field mapping
        mapping: std::collections::HashMap<String, String>,
    },
    /// Existing entities as input
    Entity {
        entity_definition: String,
        filter: EntityFilter,
        /// One-to-one mapping: source_field -> normalized_field
        mapping: std::collections::HashMap<String, String>,
    },
}

pub(crate) fn validate_from(idx: usize, from: &FromDef, safe_field: &Regex) -> Result<()> {
    match from {
        FromDef::Format {
            source,
            format,
            mapping,
        } => {
            if source.source_type.trim().is_empty() {
                bail!("DSL step {}: from.format.source.source_type must not be empty", idx);
            }
            if format.format_type.trim().is_empty() {
                bail!("DSL step {}: from.format.format.format_type must not be empty", idx);
            }
            // Validate format-specific options
            match format.format_type.as_str() {
                "csv" => {
                    if let Some(delimiter) = format.options.get("delimiter").and_then(|v| v.as_str()) {
                        if delimiter.len() != 1 {
                            bail!(
                                "DSL step {}: from.format.format.options.delimiter must be a single character",
                                idx
                            );
                        }
                    }
                    if let Some(escape) = format.options.get("escape").and_then(|v| v.as_str()) {
                        if !escape.is_empty() && escape.len() != 1 {
                            bail!(
                                "DSL step {}: from.format.format.options.escape must be a single character when set",
                                idx
                            );
                        }
                    }
                    if let Some(quote) = format.options.get("quote").and_then(|v| v.as_str()) {
                        if !quote.is_empty() && quote.len() != 1 {
                            bail!(
                                "DSL step {}: from.format.format.options.quote must be a single character when set",
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
            // Validate source config
            match source.source_type.as_str() {
                "uri" => {
                    if let Some(uri) = source.config.get("uri").and_then(|v| v.as_str()) {
                        if uri.trim().is_empty() {
                            bail!("DSL step {}: from.format.source.config.uri must not be empty", idx);
                        }
                        if !uri.starts_with("http://") && !uri.starts_with("https://") {
                            bail!("DSL step {}: from.format.source.config.uri must start with http:// or https://", idx);
                        }
                    } else {
                        bail!("DSL step {}: from.format.source.config.uri is required for uri source", idx);
                    }
                }
                "file" => {
                    // File source is handled during manual runs
                }
                "api" => {
                    if let Some(endpoint) = source.config.get("endpoint").and_then(|v| v.as_str()) {
                        if endpoint.trim().is_empty() {
                            bail!("DSL step {}: from.format.source.config.endpoint must not be empty", idx);
                        }
                        // Validate endpoint format (should start with /)
                        if !endpoint.starts_with('/') {
                            bail!("DSL step {}: from.format.source.config.endpoint must start with '/' (e.g., '/api/v1/workflows/{{uuid}}')", idx);
                        }
                    } else {
                        bail!("DSL step {}: from.format.source.config.endpoint is required for api source", idx);
                    }
                }
                _ => {
                    // Other source types will be validated by their handlers
                }
            }
            // Validate auth config if present
            if let Some(auth) = &source.auth {
                validate_auth_config(idx, auth, "from")?;
            }
            // Allow empty mappings
            validate_mapping(idx, mapping, safe_field)?;
        }
        FromDef::Entity {
            entity_definition,
            filter,
            mapping,
        } => {
            if entity_definition.trim().is_empty() {
                bail!(
                    "DSL step {}: from.entity.entity_definition must not be empty",
                    idx
                );
            }
            if filter.field.trim().is_empty() || filter.value.trim().is_empty() {
                bail!(
                    "DSL step {}: from.entity.filter requires both field and value",
                    idx
                );
            }
            // Allow empty mappings
            validate_mapping(idx, mapping, safe_field)?;
        }
    }
    Ok(())
}

/// Validate authentication configuration
fn validate_auth_config(idx: usize, auth: &AuthConfig, context: &str) -> Result<()> {
    match auth {
        AuthConfig::None => {
            // No validation needed
        }
        AuthConfig::ApiKey { key, header_name } => {
            if key.trim().is_empty() {
                bail!("DSL step {}: {}.auth.api_key.key must not be empty", idx, context);
            }
            if header_name.trim().is_empty() {
                bail!("DSL step {}: {}.auth.api_key.header_name must not be empty", idx, context);
            }
        }
        AuthConfig::BasicAuth { username, password } => {
            if username.trim().is_empty() {
                bail!("DSL step {}: {}.auth.basic_auth.username must not be empty", idx, context);
            }
            if password.trim().is_empty() {
                bail!("DSL step {}: {}.auth.basic_auth.password must not be empty", idx, context);
            }
        }
        AuthConfig::PreSharedKey { key, location: _, field_name } => {
            if key.trim().is_empty() {
                bail!("DSL step {}: {}.auth.pre_shared_key.key must not be empty", idx, context);
            }
            if field_name.trim().is_empty() {
                bail!("DSL step {}: {}.auth.pre_shared_key.field_name must not be empty", idx, context);
            }
        }
    }
    Ok(())
}

pub(crate) fn mapping_of(from: &FromDef) -> &std::collections::HashMap<String, String> {
    match from {
        FromDef::Format { mapping, .. } | FromDef::Entity { mapping, .. } => mapping,
    }
}
