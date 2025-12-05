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
            pub default_value: Option<Value>,
            #[serde(default)]
            pub validation: FieldValidation,
            #[serde(default)]
            pub ui_settings: UiSettings,
            #[serde(default)]
            pub constraints: HashMap<String, Value>,
        }

        let mut helper = FieldDefinitionHelper::deserialize(deserializer)?;

        // Extract validation fields from constraints
        if let Some(pattern) = helper.constraints.get("pattern").cloned() {
            if let Some(pattern_str) = pattern.as_str() {
                helper.validation.pattern = Some(pattern_str.to_string());
            }
        }

        if let Some(min_length) = helper.constraints.get("min_length").cloned() {
            if let Some(min_len) = min_length.as_u64() {
                // u64 to usize conversion is intentional - allow truncation on 32-bit systems
                helper.validation.min_length = usize::try_from(min_len).ok();
            }
        }

        if let Some(max_length) = helper.constraints.get("max_length").cloned() {
            if let Some(max_len) = max_length.as_u64() {
                // u64 to usize conversion is intentional - allow truncation on 32-bit systems
                helper.validation.max_length = usize::try_from(max_len).ok();
            }
        }

        if let Some(min) = helper.constraints.get("min").cloned() {
            helper.validation.min_value = Some(min);
        }

        if let Some(max) = helper.constraints.get("max").cloned() {
            helper.validation.max_value = Some(max);
        }

        if let Some(positive_only) = helper.constraints.get("positive_only").cloned() {
            if let Some(positive) = positive_only.as_bool() {
                helper.validation.positive_only = Some(positive);
            }
        }

        if let Some(min_date) = helper.constraints.get("min_date").cloned() {
            if let Some(date_str) = min_date.as_str() {
                helper.validation.min_date = Some(date_str.to_string());
            }
        }

        if let Some(max_date) = helper.constraints.get("max_date").cloned() {
            if let Some(date_str) = max_date.as_str() {
                helper.validation.max_date = Some(date_str.to_string());
            }
        }

        if let Some(target_class) = helper.constraints.get("target_class").cloned() {
            if let Some(class_str) = target_class.as_str() {
                helper.validation.target_class = Some(class_str.to_string());
            }
        }

        // Handle options source for Select/MultiSelect fields
        if let Some(options) = helper.constraints.get("options").cloned() {
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
            default_value: helper.default_value,
            validation: helper.validation,
            ui_settings: helper.ui_settings,
            constraints: helper.constraints,
        })
    }
}
