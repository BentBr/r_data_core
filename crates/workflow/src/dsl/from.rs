use crate::data::adapters::auth::AuthConfig;
use crate::dsl::validate_mapping;
use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityFilter {
    pub field: String,
    pub operator: String,
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
        /// Optional filter; if omitted, all entities are included
        #[serde(default)]
        filter: Option<EntityFilter>,
        /// One-to-one mapping: `source_field` -> `normalized_field`
        mapping: std::collections::HashMap<String, String>,
    },
    /// Read from previous step's normalized data (including calculated fields)
    /// Can only be used in steps after step 0
    PreviousStep {
        /// Field mapping from previous step's fields to this step's normalized fields
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
                bail!("DSL step {idx}: from.format.source.source_type must not be empty");
            }
            if format.format_type.trim().is_empty() {
                bail!("DSL step {idx}: from.format.format.format_type must not be empty");
            }
            // Validate format-specific options
            if format.format_type.as_str() == "csv" {
                if let Some(delimiter) = format.options.get("delimiter").and_then(|v| v.as_str()) {
                    if delimiter.len() != 1 {
                        bail!("DSL step {idx}: from.format.format.options.delimiter must be a single character");
                    }
                }
                if let Some(escape) = format.options.get("escape").and_then(|v| v.as_str()) {
                    if !escape.is_empty() && escape.len() != 1 {
                        bail!("DSL step {idx}: from.format.format.options.escape must be a single character when set");
                    }
                }
                if let Some(quote) = format.options.get("quote").and_then(|v| v.as_str()) {
                    if !quote.is_empty() && quote.len() != 1 {
                        bail!("DSL step {idx}: from.format.format.options.quote must be a single character when set");
                    }
                }
            } else {
                // JSON format and other formats have minimal validation
                // Other formats will be validated by their handlers
            }
            // Validate source config
            #[allow(clippy::match_same_arms)] // "file" and "_" have different semantic meanings
            match source.source_type.as_str() {
                "uri" => {
                    if let Some(uri) = source.config.get("uri").and_then(|v| v.as_str()) {
                        if uri.trim().is_empty() {
                            bail!(
                                "DSL step {idx}: from.format.source.config.uri must not be empty"
                            );
                        }
                        if !uri.starts_with("http://") && !uri.starts_with("https://") {
                            bail!("DSL step {idx}: from.format.source.config.uri must start with http:// or https://");
                        }
                    } else {
                        bail!("DSL step {idx}: from.format.source.config.uri is required for uri source");
                    }
                }
                "file" => {
                    // File source is handled during manual runs
                }
                "api" => {
                    // from.api source type = Accept data via POST to this workflow
                    // No endpoint field needed (always /api/v1/workflows/{this-workflow-uuid})
                    // If endpoint field is present, it's invalid (use from.uri instead to pull from provider workflows)
                    if source.config.get("endpoint").is_some() {
                        bail!("DSL step {idx}: from.format.source.config.endpoint is not allowed for 'api' source type. Use 'uri' source type to pull from provider workflows.");
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
                bail!("DSL step {idx}: from.entity.entity_definition must not be empty");
            }
            if let Some(filter) = filter {
                if filter.field.trim().is_empty()
                    || filter.value.trim().is_empty()
                    || filter.operator.trim().is_empty()
                {
                    bail!("DSL step {idx}: from.entity.filter requires field, operator, and value");
                }
                // Validate filter field name is safe (prevents SQL injection)
                if !safe_field.is_match(&filter.field) {
                    bail!(
                        "DSL step {idx}: from.entity.filter.field must be a safe identifier (got: '{}')",
                        filter.field
                    );
                }
                // Validate operator is one of the allowed values
                let allowed_operators = ["=", ">", "<", "<=", ">=", "IN", "NOT IN"];
                if !allowed_operators.contains(&filter.operator.as_str()) {
                    bail!(
                        "DSL step {idx}: from.entity.filter.operator must be one of: =, >, <, <=, >=, IN, NOT IN"
                    );
                }
            }
            // Allow empty mappings
            validate_mapping(idx, mapping, safe_field)?;
        }
        FromDef::PreviousStep { mapping } => {
            // PreviousStep can only be used in steps after step 0
            if idx == 0 {
                bail!("DSL step {idx}: from.previous_step cannot be used in the first step (step 0). The first step must read from a Format or Entity source.");
            }
            // Allow empty mappings (pass through all fields from previous step)
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
                bail!("DSL step {idx}: {context}.auth.api_key.key must not be empty");
            }
            if header_name.trim().is_empty() {
                bail!("DSL step {idx}: {context}.auth.api_key.header_name must not be empty");
            }
        }
        AuthConfig::BasicAuth { username, password } => {
            if username.trim().is_empty() {
                bail!("DSL step {idx}: {context}.auth.basic_auth.username must not be empty");
            }
            if password.trim().is_empty() {
                bail!("DSL step {idx}: {context}.auth.basic_auth.password must not be empty");
            }
        }
        AuthConfig::PreSharedKey {
            key,
            location: _,
            field_name,
        } => {
            if key.trim().is_empty() {
                bail!("DSL step {idx}: {context}.auth.pre_shared_key.key must not be empty");
            }
            if field_name.trim().is_empty() {
                bail!("DSL step {idx}: {context}.auth.pre_shared_key.field_name must not be empty");
            }
        }
    }
    Ok(())
}

#[allow(clippy::missing_const_for_fn)] // Cannot be const due to pattern matching
pub(crate) fn mapping_of(from: &FromDef) -> &std::collections::HashMap<String, String> {
    match from {
        FromDef::Format { mapping, .. }
        | FromDef::Entity { mapping, .. }
        | FromDef::PreviousStep { mapping } => mapping,
    }
}
