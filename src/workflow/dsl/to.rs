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
        /// Mapping from normalized_field -> destination_field
        mapping: std::collections::HashMap<String, String>,
    },
}

pub(crate) fn validate_to(idx: usize, to: &ToDef, safe_field: &Regex) -> Result<()> {
    match to {
        ToDef::Csv { output: _, mapping } | ToDef::Json { output: _, mapping } => {
            validate_mapping(idx, mapping, safe_field)?;
        }
        ToDef::Entity {
            entity_definition,
            path,
            mode: _,
            identify: _,
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
