#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::admin::entity_definitions::models::{
    DateTimeConstraints, EntityDefinitionSchema, FieldConstraints, FieldDefinitionSchema,
    FieldTypeSchema, NumericConstraints, RelationConstraints, SchemaConstraints, SelectConstraints,
    StringConstraints, UiSettingsSchema,
};
use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::field::FieldDefinition;
use r_data_core_core::field::FieldType;
use time::format_description::well_known::Rfc3339;

/// Convert `EntityDefinition` to API schema model
pub fn entity_definition_to_schema_model(def: &EntityDefinition) -> EntityDefinitionSchema {
    EntityDefinitionSchema {
        uuid: Some(def.uuid),
        entity_type: def.entity_type.clone(),
        display_name: def.display_name.clone(),
        description: def.description.clone(),
        group_name: def.group_name.clone(),
        allow_children: def.allow_children,
        icon: def.icon.clone(),
        fields: def
            .fields
            .iter()
            .map(field_definition_to_schema_model)
            .collect(),
        published: Some(def.published),
        created_at: Some(def.created_at.format(&Rfc3339).unwrap_or_default()),
        updated_at: Some(def.updated_at.format(&Rfc3339).unwrap_or_default()),
    }
}

/// Convert `FieldDefinition` to API schema model
#[allow(clippy::too_many_lines)] // Conversion function with many field mappings
pub fn field_definition_to_schema_model(field: &FieldDefinition) -> FieldDefinitionSchema {
    let field_type_clone = field.field_type.clone();
    let field_type = field_type_to_schema(&field_type_clone);

    let constraints = match field_type_clone {
        FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
            FieldConstraints::String(StringConstraints {
                min_length: field.validation.min_length,
                max_length: field.validation.max_length,
                pattern: field.validation.pattern.clone(),
                error_message: None,
            })
        }
        FieldType::Integer => FieldConstraints::Integer(NumericConstraints {
            min: field
                .validation
                .min_value
                .as_ref()
                .and_then(serde_json::Value::as_f64),
            max: field
                .validation
                .max_value
                .as_ref()
                .and_then(serde_json::Value::as_f64),
            precision: None,
            positive_only: field.validation.positive_only,
        }),
        FieldType::Float => FieldConstraints::Float(NumericConstraints {
            min: field
                .validation
                .min_value
                .as_ref()
                .and_then(serde_json::Value::as_f64),
            max: field
                .validation
                .max_value
                .as_ref()
                .and_then(serde_json::Value::as_f64),
            precision: None,
            positive_only: field.validation.positive_only,
        }),
        FieldType::DateTime | FieldType::Date => FieldConstraints::DateTime(DateTimeConstraints {
            min_date: field.validation.min_date.clone(),
            max_date: field.validation.max_date.clone(),
        }),
        FieldType::ManyToOne | FieldType::ManyToMany => {
            FieldConstraints::Relation(RelationConstraints {
                target_class: field.validation.target_class.clone().unwrap_or_default(),
            })
        }
        FieldType::Select => {
            // Extract options from OptionsSource if it's Fixed
            let options =
                field
                    .validation
                    .options_source
                    .as_ref()
                    .and_then(|source| match source {
                        r_data_core_core::field::options::OptionsSource::Fixed { options } => {
                            Some(options.iter().map(|opt| opt.value.clone()).collect())
                        }
                        _ => None,
                    });
            FieldConstraints::Select(SelectConstraints { options })
        }
        FieldType::MultiSelect => {
            // Extract options from OptionsSource if it's Fixed
            let options =
                field
                    .validation
                    .options_source
                    .as_ref()
                    .and_then(|source| match source {
                        r_data_core_core::field::options::OptionsSource::Fixed { options } => {
                            Some(options.iter().map(|opt| opt.value.clone()).collect())
                        }
                        _ => None,
                    });
            FieldConstraints::MultiSelect(SelectConstraints { options })
        }
        _ => FieldConstraints::Schema(SchemaConstraints {
            schema: serde_json::json!({}),
        }),
    };

    FieldDefinitionSchema {
        name: field.name.clone(),
        display_name: field.display_name.clone(),
        field_type,
        description: field.description.clone(),
        required: field.required,
        indexed: field.indexed,
        filterable: field.filterable,
        unique: field.unique,
        default_value: field.default_value.clone(),
        constraints: Some(constraints),
        ui_settings: UiSettingsSchema {
            placeholder: field.ui_settings.placeholder.clone(),
            help_text: field.ui_settings.help_text.clone(),
            hide_in_lists: field.ui_settings.hide_in_lists,
            width: field.ui_settings.width,
            order: field.ui_settings.order,
            group: field.ui_settings.group.clone(),
            css_class: field.ui_settings.css_class.clone(),
            wysiwyg_toolbar: field.ui_settings.wysiwyg_toolbar.clone(),
            input_type: field.ui_settings.input_type.clone(),
        },
    }
}

/// Convert `FieldType` to `FieldTypeSchema`
#[must_use]
pub const fn field_type_to_schema(field_type: &FieldType) -> FieldTypeSchema {
    match *field_type {
        FieldType::String => FieldTypeSchema::String,
        FieldType::Text => FieldTypeSchema::Text,
        FieldType::Wysiwyg => FieldTypeSchema::Wysiwyg,
        FieldType::Integer => FieldTypeSchema::Integer,
        FieldType::Float => FieldTypeSchema::Float,
        FieldType::Boolean => FieldTypeSchema::Boolean,
        FieldType::DateTime => FieldTypeSchema::DateTime,
        FieldType::Date => FieldTypeSchema::Date,
        FieldType::Json => FieldTypeSchema::Json,
        FieldType::Object => FieldTypeSchema::Object,
        FieldType::Array => FieldTypeSchema::Array,
        FieldType::Uuid => FieldTypeSchema::Uuid,
        FieldType::ManyToOne => FieldTypeSchema::ManyToOne,
        FieldType::ManyToMany => FieldTypeSchema::ManyToMany,
        FieldType::Select => FieldTypeSchema::Select,
        FieldType::MultiSelect => FieldTypeSchema::MultiSelect,
        FieldType::Image => FieldTypeSchema::Image,
        FieldType::File => FieldTypeSchema::File,
        FieldType::Password => FieldTypeSchema::Password,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_field_type_converts_to_json_schema() {
        let result = field_type_to_schema(&FieldType::Json);
        assert!(matches!(result, FieldTypeSchema::Json));
    }

    #[test]
    fn test_object_field_type_converts_to_object_schema() {
        let result = field_type_to_schema(&FieldType::Object);
        assert!(matches!(result, FieldTypeSchema::Object));
    }

    #[test]
    fn test_json_and_object_are_distinct() {
        let json_result = field_type_to_schema(&FieldType::Json);
        let object_result = field_type_to_schema(&FieldType::Object);

        // They should be different variants
        assert!(matches!(json_result, FieldTypeSchema::Json));
        assert!(matches!(object_result, FieldTypeSchema::Object));
    }

    #[test]
    fn test_array_field_type_converts_to_array_schema() {
        let result = field_type_to_schema(&FieldType::Array);
        assert!(matches!(result, FieldTypeSchema::Array));
    }

    #[test]
    fn test_all_field_types_have_schema_mappings() {
        // Ensure all field types can be converted without panic
        let field_types = [
            FieldType::String,
            FieldType::Text,
            FieldType::Wysiwyg,
            FieldType::Integer,
            FieldType::Float,
            FieldType::Boolean,
            FieldType::DateTime,
            FieldType::Date,
            FieldType::Object,
            FieldType::Array,
            FieldType::Uuid,
            FieldType::Json,
            FieldType::ManyToOne,
            FieldType::ManyToMany,
            FieldType::Select,
            FieldType::MultiSelect,
            FieldType::Image,
            FieldType::File,
        ];

        for field_type in field_types {
            // This should not panic
            let _ = field_type_to_schema(&field_type);
        }
    }

    #[test]
    fn test_json_schema_serializes_as_json() {
        let json_schema = FieldTypeSchema::Json;
        let serialized = serde_json::to_string(&json_schema).unwrap();
        assert_eq!(serialized, "\"Json\"");
    }

    #[test]
    fn test_object_schema_serializes_as_object() {
        let object_schema = FieldTypeSchema::Object;
        let serialized = serde_json::to_string(&object_schema).unwrap();
        assert_eq!(serialized, "\"Object\"");
    }

    #[test]
    fn test_json_schema_deserializes_from_json() {
        let json_schema: FieldTypeSchema = serde_json::from_str("\"Json\"").unwrap();
        assert!(matches!(json_schema, FieldTypeSchema::Json));
    }

    #[test]
    fn test_object_schema_deserializes_from_object() {
        let object_schema: FieldTypeSchema = serde_json::from_str("\"Object\"").unwrap();
        assert!(matches!(object_schema, FieldTypeSchema::Object));
    }
}
