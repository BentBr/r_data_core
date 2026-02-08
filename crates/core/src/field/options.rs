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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_validation_with_string_constraints() {
        let validation = FieldValidation {
            min_length: Some(5),
            max_length: Some(100),
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        };

        assert_eq!(validation.min_length, Some(5));
        assert_eq!(validation.max_length, Some(100));
        assert_eq!(validation.pattern, Some("^[a-z]+$".to_string()));
    }

    #[test]
    fn test_field_validation_defaults_to_none() {
        let validation = FieldValidation::default();
        assert_eq!(validation.min_length, None);
        assert_eq!(validation.max_length, None);
        assert_eq!(validation.pattern, None);
        assert_eq!(validation.min_value, None);
        assert_eq!(validation.max_value, None);
        assert_eq!(validation.positive_only, None);
    }

    #[test]
    fn test_field_validation_serialization() {
        let validation = FieldValidation {
            min_length: Some(5),
            max_length: Some(100),
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&validation).unwrap();
        assert!(json.contains("\"min_length\":5"));
        assert!(json.contains("\"max_length\":100"));
        assert!(json.contains("\"pattern\":\"^[a-z]+$\""));

        let deserialized: FieldValidation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.min_length, Some(5));
        assert_eq!(deserialized.max_length, Some(100));
        assert_eq!(deserialized.pattern, Some("^[a-z]+$".to_string()));
    }

    #[test]
    fn test_field_validation_numeric_constraints() {
        let validation = FieldValidation {
            min_value: Some(serde_json::json!(0)),
            max_value: Some(serde_json::json!(100)),
            positive_only: Some(true),
            ..Default::default()
        };

        assert_eq!(validation.min_value, Some(serde_json::json!(0)));
        assert_eq!(validation.max_value, Some(serde_json::json!(100)));
        assert_eq!(validation.positive_only, Some(true));
    }
}
