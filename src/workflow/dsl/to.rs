use crate::workflow::dsl::validate_mapping;
use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Api,
    Download,
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
    /// CSV output (either delivered via API or as a downloadable artifact)
    Csv {
        output: OutputMode,
        /// CSV writing options (header/delimiter/escape/quote)
        #[serde(default)]
        options: super::CsvOptions,
        /// Mapping from normalized_field -> destination_field
        mapping: std::collections::HashMap<String, String>,
    },
    /// JSON output (either delivered via API or as a downloadable artifact)
    Json {
        output: OutputMode,
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
        ToDef::Csv {
            output: _,
            options,
            mapping,
        } => {
            // Validate CSV options lengths
            if options.delimiter.len() != 1 {
                bail!(
                    "DSL step {}: to.csv.options.delimiter must be a single character",
                    idx
                );
            }
            if let Some(esc) = &options.escape {
                if !esc.is_empty() && esc.len() != 1 {
                    bail!(
                        "DSL step {}: to.csv.options.escape must be a single character when set",
                        idx
                    );
                }
            }
            if let Some(q) = &options.quote {
                if !q.is_empty() && q.len() != 1 {
                    bail!(
                        "DSL step {}: to.csv.options.quote must be a single character when set",
                        idx
                    );
                }
            }
            // Allow empty mappings
            validate_mapping(idx, mapping, safe_field)?;
        }
        ToDef::Json { output: _, mapping } => {
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
        ToDef::Csv { mapping, .. }
        | ToDef::Json { mapping, .. }
        | ToDef::Entity { mapping, .. } => mapping,
    }
}
