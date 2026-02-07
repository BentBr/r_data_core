use serde::{Deserialize, Serialize};
use serde_json;

/// Source of options for select fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

    /// Whether this field must have unique values within the entity type
    pub unique: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_validation_unique_serialization() {
        let validation = FieldValidation {
            unique: Some(true),
            ..Default::default()
        };

        let json = serde_json::to_string(&validation).unwrap();
        assert!(json.contains("\"unique\":true"));

        let deserialized: FieldValidation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.unique, Some(true));
    }

    #[test]
    fn test_field_validation_unique_defaults_to_none() {
        let validation = FieldValidation::default();
        assert_eq!(validation.unique, None);
    }

    #[test]
    fn test_field_validation_with_all_string_constraints() {
        let validation = FieldValidation {
            min_length: Some(5),
            max_length: Some(100),
            pattern: Some("^[a-z]+$".to_string()),
            unique: Some(true),
            ..Default::default()
        };

        assert_eq!(validation.min_length, Some(5));
        assert_eq!(validation.max_length, Some(100));
        assert_eq!(validation.pattern, Some("^[a-z]+$".to_string()));
        assert_eq!(validation.unique, Some(true));
    }

    #[test]
    fn test_field_validation_unique_false_serialization() {
        let validation = FieldValidation {
            unique: Some(false),
            ..Default::default()
        };

        let json = serde_json::to_string(&validation).unwrap();
        assert!(json.contains("\"unique\":false"));

        let deserialized: FieldValidation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.unique, Some(false));
    }
}
