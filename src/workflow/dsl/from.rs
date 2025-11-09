use crate::workflow::dsl::validate_mapping;
use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityFilter {
    pub field: String,
    pub value: String,
}

/// FROM definitions - where data originates
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromDef {
    /// CSV input (uri is replaced with uploaded file during manual runs)
    Csv {
        uri: String,
        /// CSV parsing options (header/delimiter/escape/quote)
        #[serde(default)]
        options: super::CsvOptions,
        /// One-to-one mapping: source_field -> normalized_field
        mapping: std::collections::HashMap<String, String>,
    },
    /// JSON/NDJSON input (uri is replaced with uploaded file during manual runs)
    Json {
        uri: String,
        /// One-to-one mapping: source_field -> normalized_field
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
        FromDef::Csv {
            uri,
            options,
            mapping,
        } => {
            if uri.trim().is_empty() {
                bail!("DSL step {}: from.uri must not be empty", idx);
            }
            if options.delimiter.len() != 1 {
                bail!(
                    "DSL step {}: from.csv.options.delimiter must be a single character",
                    idx
                );
            }
            if let Some(esc) = &options.escape {
                if !esc.is_empty() && esc.len() != 1 {
                    bail!(
                        "DSL step {}: from.csv.options.escape must be a single character when set",
                        idx
                    );
                }
            }
            if let Some(q) = &options.quote {
                if !q.is_empty() && q.len() != 1 {
                    bail!(
                        "DSL step {}: from.csv.options.quote must be a single character when set",
                        idx
                    );
                }
            }
            // Allow empty mappings
            validate_mapping(idx, mapping, safe_field)?;
        }
        FromDef::Json { uri, mapping } => {
            if uri.trim().is_empty() {
                bail!("DSL step {}: from.uri must not be empty", idx);
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

pub(crate) fn mapping_of(from: &FromDef) -> &std::collections::HashMap<String, String> {
    match from {
        FromDef::Csv { mapping, .. }
        | FromDef::Json { mapping, .. }
        | FromDef::Entity { mapping, .. } => mapping,
    }
}
