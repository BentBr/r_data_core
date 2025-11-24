use r_data_core_core::field::definition::FieldDefinition;
use r_data_core_core::field::types::FieldType;

impl FieldDefinition {
    /// Convert to API schema model with properly typed constraints
    pub fn to_schema_model(
        &self,
    ) -> crate::api::admin::entity_definitions::models::FieldDefinitionSchema {
        use crate::api::admin::entity_definitions::models::{
            DateTimeConstraints, FieldConstraints, FieldDefinitionSchema, FieldTypeSchema,
            NumericConstraints, RelationConstraints, SchemaConstraints, SelectConstraints,
            StringConstraints, UiSettingsSchema,
        };

        // Determine the field type
        let field_type = match self.field_type {
            FieldType::String => FieldTypeSchema::String,
            FieldType::Text => FieldTypeSchema::Text,
            FieldType::Wysiwyg => FieldTypeSchema::Wysiwyg,
            FieldType::Integer => FieldTypeSchema::Integer,
            FieldType::Float => FieldTypeSchema::Float,
            FieldType::Boolean => FieldTypeSchema::Boolean,
            FieldType::DateTime => FieldTypeSchema::DateTime,
            FieldType::Date => FieldTypeSchema::Date,
            FieldType::Json => FieldTypeSchema::Object, // Map Json to Object
            FieldType::Object => FieldTypeSchema::Object,
            FieldType::Array => FieldTypeSchema::Array,
            FieldType::Uuid => FieldTypeSchema::Uuid,
            FieldType::ManyToOne => FieldTypeSchema::ManyToOne,
            FieldType::ManyToMany => FieldTypeSchema::ManyToMany,
            FieldType::Select => FieldTypeSchema::Select,
            FieldType::MultiSelect => FieldTypeSchema::MultiSelect,
            FieldType::Image => FieldTypeSchema::Image,
            FieldType::File => FieldTypeSchema::File,
        };

        // Convert constraints to typed constraints
        let constraints = match self.field_type {
            FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
                let string_constraints = StringConstraints {
                    min_length: self.validation.min_length,
                    max_length: self.validation.max_length,
                    pattern: self.validation.pattern.clone(),
                    error_message: self
                        .constraints
                        .get("error_message")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };

                if string_constraints.min_length.is_some()
                    || string_constraints.max_length.is_some()
                    || string_constraints.pattern.is_some()
                    || string_constraints.error_message.is_some()
                {
                    Some(FieldConstraints::String(string_constraints))
                } else {
                    None
                }
            }
            FieldType::Integer | FieldType::Float => {
                let numeric_constraints = NumericConstraints {
                    min: self.validation.min_value.as_ref().and_then(|v| v.as_f64()),
                    max: self.validation.max_value.as_ref().and_then(|v| v.as_f64()),
                    precision: self
                        .constraints
                        .get("precision")
                        .and_then(|v| v.as_u64())
                        .map(|p| p as u8),
                    positive_only: self.validation.positive_only,
                };

                if numeric_constraints.min.is_some()
                    || numeric_constraints.max.is_some()
                    || numeric_constraints.precision.is_some()
                    || numeric_constraints.positive_only.is_some()
                {
                    if self.field_type == FieldType::Integer {
                        Some(FieldConstraints::Integer(numeric_constraints))
                    } else {
                        Some(FieldConstraints::Float(numeric_constraints))
                    }
                } else {
                    None
                }
            }
            FieldType::DateTime | FieldType::Date => {
                let datetime_constraints = DateTimeConstraints {
                    min_date: self.validation.min_date.clone(),
                    max_date: self.validation.max_date.clone(),
                };

                if datetime_constraints.min_date.is_some()
                    || datetime_constraints.max_date.is_some()
                {
                    if self.field_type == FieldType::DateTime {
                        Some(FieldConstraints::DateTime(datetime_constraints))
                    } else {
                        Some(FieldConstraints::Date(datetime_constraints))
                    }
                } else {
                    None
                }
            }
            FieldType::Select | FieldType::MultiSelect => {
                // Extract options from options_source
                let options = match &self.validation.options_source {
                    Some(source) => match source {
                        r_data_core_core::field::options::OptionsSource::Fixed { options } => {
                            Some(options.iter().map(|opt| opt.value.clone()).collect())
                        }
                        _ => None, // Handle other option sources
                    },
                    None => self
                        .constraints
                        .get("options")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        }),
                };

                if let Some(opts) = options {
                    let select_constraints = SelectConstraints {
                        options: Some(opts),
                    };

                    if self.field_type == FieldType::Select {
                        Some(FieldConstraints::Select(select_constraints))
                    } else {
                        Some(FieldConstraints::MultiSelect(select_constraints))
                    }
                } else {
                    None
                }
            }
            FieldType::ManyToOne | FieldType::ManyToMany => {
                if let Some(target) = &self.validation.target_class {
                    let relation_constraints = RelationConstraints {
                        target_class: target.clone(),
                    };
                    Some(FieldConstraints::Relation(relation_constraints))
                } else {
                    None
                }
            }
            FieldType::Object | FieldType::Array | FieldType::Json => {
                if let Some(schema) = self.constraints.get("schema") {
                    let schema_constraints = SchemaConstraints {
                        schema: schema.clone(),
                    };
                    Some(FieldConstraints::Schema(schema_constraints))
                } else {
                    None
                }
            }
            _ => None,
        };

        // Convert UI settings
        let ui_settings = UiSettingsSchema {
            placeholder: self.ui_settings.placeholder.clone(),
            help_text: self.ui_settings.help_text.clone(),
            hide_in_lists: self.ui_settings.hide_in_lists,
            width: self.ui_settings.width,
            order: self.ui_settings.order,
            group: self.ui_settings.group.clone(),
            css_class: self.ui_settings.css_class.clone(),
            wysiwyg_toolbar: self.ui_settings.wysiwyg_toolbar.clone(),
            input_type: self.ui_settings.input_type.clone(),
        };

        FieldDefinitionSchema {
            name: self.name.clone(),
            display_name: self.display_name.clone(),
            field_type,
            description: self.description.clone(),
            required: self.required,
            indexed: self.indexed,
            filterable: self.filterable,
            default_value: self.default_value.clone(),
            constraints,
            ui_settings,
        }
    }
}
