use serde::{Deserialize, Serialize};
use serde_json;

/// Source of options for select fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum OptionsSource {
    /// Fixed list of options
    #[serde(rename = "fixed")]
    Fixed { options: Vec<SelectOption> },

    /// Options from an enum stored in database
    #[serde(rename = "enum")]
    Enum { enum_name: String },

    /// Options from a database query
    #[serde(rename = "query")]
    Query {
        entity_type: String,
        value_field: String,
        label_field: String,
        filter: Option<serde_json::Value>,
    },
}

/// Option for select fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// Validation rules for fields
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldValidation {
    /// Minimum string length
    pub min_length: Option<usize>,

    /// Maximum string length
    pub max_length: Option<usize>,

    /// Regex pattern for string validation
    pub pattern: Option<String>,

    /// Minimum numeric value
    pub min_value: Option<serde_json::Value>,

    /// Maximum numeric value
    pub max_value: Option<serde_json::Value>,

    /// Allow only positive values for numeric fields
    pub positive_only: Option<bool>,

    /// Minimum date (ISO string or "now")
    pub min_date: Option<String>,

    /// Maximum date (ISO string or "now")
    pub max_date: Option<String>,

    /// For relation fields: target entity class
    pub target_class: Option<String>,

    /// For select fields: options source
    pub options_source: Option<OptionsSource>,
}
