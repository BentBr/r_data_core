use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashMap;

use crate::field::definition::FieldDefinition;
use crate::field::options::FieldValidation;
use crate::field::options::{OptionsSource, SelectOption};
use crate::field::types::FieldType;
use crate::field::ui::UiSettings;

// Manual implementation of Deserialize for FieldDefinition to handle constraints
impl<'de> Deserialize<'de> for FieldDefinition {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct FieldDefinitionHelper {
            pub name: String,
            pub display_name: String,
            pub field_type: FieldType,
            pub description: Option<String>,
            pub required: bool,
            pub indexed: bool,
            #[serde(default)]
            pub filterable: bool,
            #[serde(default)]
            pub unique: bool,
            pub default_value: Option<Value>,
            #[serde(default)]
            pub validation: FieldValidation,
            #[serde(default)]
            pub ui_settings: UiSettings,
            #[serde(default)]
            pub constraints: HashMap<String, Value>,
        }

        let mut helper = FieldDefinitionHelper::deserialize(deserializer)?;

        // Extract validation fields from nested constraints structure
        // API format: { type: "string", constraints: { pattern, min_length, ... } }
        let inner_constraints: HashMap<String, Value> = helper
            .constraints
            .get("constraints")
            .and_then(|nested| nested.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        if let Some(pattern) = inner_constraints.get("pattern").cloned() {
            if let Some(pattern_str) = pattern.as_str() {
                helper.validation.pattern = Some(pattern_str.to_string());
            }
        }

        if let Some(min_length) = inner_constraints.get("min_length").cloned() {
            if let Some(min_len) = min_length.as_u64() {
                // u64 to usize conversion is intentional - allow truncation on 32-bit systems
                helper.validation.min_length = usize::try_from(min_len).ok();
            }
        }

        if let Some(max_length) = inner_constraints.get("max_length").cloned() {
            if let Some(max_len) = max_length.as_u64() {
                // u64 to usize conversion is intentional - allow truncation on 32-bit systems
                helper.validation.max_length = usize::try_from(max_len).ok();
            }
        }

        if let Some(min) = inner_constraints.get("min").cloned() {
            helper.validation.min_value = Some(min);
        }

        if let Some(max) = inner_constraints.get("max").cloned() {
            helper.validation.max_value = Some(max);
        }

        if let Some(positive_only) = inner_constraints.get("positive_only").cloned() {
            if let Some(positive) = positive_only.as_bool() {
                helper.validation.positive_only = Some(positive);
            }
        }

        if let Some(min_date) = inner_constraints.get("min_date").cloned() {
            if let Some(date_str) = min_date.as_str() {
                helper.validation.min_date = Some(date_str.to_string());
            }
        }

        if let Some(max_date) = inner_constraints.get("max_date").cloned() {
            if let Some(date_str) = max_date.as_str() {
                helper.validation.max_date = Some(date_str.to_string());
            }
        }

        if let Some(target_class) = inner_constraints.get("target_class").cloned() {
            if let Some(class_str) = target_class.as_str() {
                helper.validation.target_class = Some(class_str.to_string());
            }
        }

        // Handle options source for Select/MultiSelect fields
        if let Some(options) = inner_constraints.get("options").cloned() {
            if let Some(options_array) = options.as_array() {
                let mut select_options = Vec::new();
                for opt in options_array {
                    if let Some(opt_str) = opt.as_str() {
                        select_options.push(SelectOption {
                            value: opt_str.to_string(),
                            label: opt_str.to_string(),
                        });
                    }
                }

                if !select_options.is_empty() {
                    helper.validation.options_source = Some(OptionsSource::Fixed {
                        options: select_options,
                    });
                }
            }
        }

        Ok(Self {
            name: helper.name,
            display_name: helper.display_name,
            field_type: helper.field_type,
            description: helper.description,
            required: helper.required,
            indexed: helper.indexed,
            filterable: helper.filterable,
            unique: helper.unique,
            default_value: helper.default_value,
            validation: helper.validation,
            ui_settings: helper.ui_settings,
            constraints: helper.constraints,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_nested_constraints_pattern() {
        // This is the structure sent by the frontend
        let json = r#"{
            "name": "email",
            "display_name": "Email",
            "field_type": "String",
            "required": true,
            "indexed": true,
            "filterable": true,
            "unique": true,
            "constraints": {
                "type": "string",
                "constraints": {
                    "pattern": "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$",
                    "min_length": 5,
                    "max_length": 100
                }
            }
        }"#;

        let field: FieldDefinition = serde_json::from_str(json).unwrap();

        assert_eq!(field.name, "email");
        assert!(field.unique);
        // Nested constraints should be extracted to validation
        assert_eq!(
            field.validation.pattern,
            Some("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$".to_string())
        );
        assert_eq!(field.validation.min_length, Some(5));
        assert_eq!(field.validation.max_length, Some(100));
    }

    #[test]
    fn test_deserialize_nested_constraints_null_values() {
        // Nested structure with null values (as sent by frontend)
        let json = r#"{
            "name": "name",
            "display_name": "Name",
            "field_type": "String",
            "required": false,
            "indexed": false,
            "filterable": false,
            "constraints": {
                "type": "string",
                "constraints": {
                    "pattern": null,
                    "min_length": null,
                    "max_length": null
                }
            }
        }"#;

        let field: FieldDefinition = serde_json::from_str(json).unwrap();

        assert_eq!(field.name, "name");
        // Null values should result in None
        assert_eq!(field.validation.pattern, None);
        assert_eq!(field.validation.min_length, None);
        assert_eq!(field.validation.max_length, None);
    }

    #[test]
    fn test_deserialize_nested_numeric_constraints() {
        let json = r#"{
            "name": "age",
            "display_name": "Age",
            "field_type": "Integer",
            "required": true,
            "indexed": false,
            "filterable": false,
            "constraints": {
                "type": "integer",
                "constraints": {
                    "min": 0,
                    "max": 150,
                    "positive_only": true
                }
            }
        }"#;

        let field: FieldDefinition = serde_json::from_str(json).unwrap();

        assert_eq!(field.name, "age");
        assert_eq!(field.validation.min_value, Some(serde_json::json!(0)));
        assert_eq!(field.validation.max_value, Some(serde_json::json!(150)));
        assert_eq!(field.validation.positive_only, Some(true));
    }
}
