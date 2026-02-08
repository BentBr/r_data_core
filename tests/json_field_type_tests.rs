#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Integration tests for JSON field type validation (Json, Object, Array)
//!
//! These tests verify that:
//! - `Json` field type accepts any valid JSON value (objects, arrays, strings, numbers, booleans, null)
//! - `Object` field type only accepts JSON objects (rejects arrays and primitives)
//! - `Array` field type only accepts JSON arrays (rejects objects and primitives)

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::entity_definition::schema::Schema;
use r_data_core_core::field::options::FieldValidation;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldType};
use r_data_core_core::DynamicEntity;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Helper to create an entity definition with a single field of the specified type
fn create_entity_definition_with_field(
    field_name: &str,
    field_type: FieldType,
) -> EntityDefinition {
    let mut schema_properties = HashMap::new();
    schema_properties.insert(
        "entity_type".to_string(),
        serde_json::Value::String("TestEntity".to_string()),
    );

    EntityDefinition {
        uuid: Uuid::new_v4(),
        entity_type: "TestEntity".to_string(),
        display_name: "Test Entity".to_string(),
        description: Some("Test entity for JSON field validation".to_string()),
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![FieldDefinition {
            name: field_name.to_string(),
            display_name: field_name.to_string(),
            field_type,
            description: None,
            required: false,
            indexed: false,
            filterable: false,
            unique: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        }],
        schema: Schema::new(schema_properties),
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
        created_by: Uuid::nil(),
        updated_by: None,
        published: true,
        version: 1,
    }
}

/// Helper to create a dynamic entity with the given field data
fn create_test_entity(
    field_name: &str,
    value: serde_json::Value,
    entity_def: EntityDefinition,
) -> DynamicEntity {
    let mut data = HashMap::new();
    data.insert(field_name.to_string(), value);
    DynamicEntity::from_data("TestEntity".to_string(), data, Arc::new(entity_def))
}

#[cfg(test)]
mod json_field_type_tests {
    use super::*;

    // =========================================================================
    // POSITIVE TESTS: Json field type accepts any valid JSON value
    // =========================================================================

    #[test]
    fn test_json_field_accepts_object() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", json!({"key": "value"}), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Json field should accept object: {result:?}"
        );
    }

    #[test]
    fn test_json_field_accepts_array() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", json!(["item1", "item2"]), entity_def);

        let result = entity.validate();
        assert!(result.is_ok(), "Json field should accept array: {result:?}");
    }

    #[test]
    fn test_json_field_accepts_array_of_objects() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity(
            "data",
            json!([
                {"entity_type": "Customer", "count": 100},
                {"entity_type": "Product", "count": 50}
            ]),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Json field should accept array of objects: {result:?}"
        );
    }

    #[test]
    fn test_json_field_accepts_string() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", json!("simple string"), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Json field should accept string: {result:?}"
        );
    }

    #[test]
    fn test_json_field_accepts_number() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", json!(42), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Json field should accept number: {result:?}"
        );
    }

    #[test]
    fn test_json_field_accepts_float() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", json!(84.234), entity_def);

        let result = entity.validate();
        assert!(result.is_ok(), "Json field should accept float: {result:?}");
    }

    #[test]
    fn test_json_field_accepts_boolean_true() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", json!(true), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Json field should accept boolean true: {result:?}"
        );
    }

    #[test]
    fn test_json_field_accepts_boolean_false() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", json!(false), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Json field should accept boolean false: {result:?}"
        );
    }

    #[test]
    fn test_json_field_accepts_null() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", serde_json::Value::Null, entity_def);

        let result = entity.validate();
        assert!(result.is_ok(), "Json field should accept null: {result:?}");
    }

    #[test]
    fn test_json_field_accepts_nested_structure() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity(
            "data",
            json!({
                "nested": {
                    "array": [1, 2, 3],
                    "object": {"key": "value"}
                }
            }),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Json field should accept nested structure: {result:?}"
        );
    }

    #[test]
    fn test_json_field_accepts_empty_array() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", json!([]), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Json field should accept empty array: {result:?}"
        );
    }

    #[test]
    fn test_json_field_accepts_empty_object() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let entity = create_test_entity("data", json!({}), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Json field should accept empty object: {result:?}"
        );
    }

    // =========================================================================
    // POSITIVE TESTS: Object field type accepts JSON objects
    // =========================================================================

    #[test]
    fn test_object_field_accepts_object() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let entity = create_test_entity("data", json!({"key": "value"}), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Object field should accept object: {result:?}"
        );
    }

    #[test]
    fn test_object_field_accepts_empty_object() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let entity = create_test_entity("data", json!({}), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Object field should accept empty object: {result:?}"
        );
    }

    #[test]
    fn test_object_field_accepts_nested_object() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let entity = create_test_entity(
            "data",
            json!({
                "level1": {
                    "level2": {
                        "key": "deeply nested"
                    }
                }
            }),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Object field should accept nested object: {result:?}"
        );
    }

    #[test]
    fn test_object_field_accepts_object_with_array_values() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let entity = create_test_entity(
            "data",
            json!({
                "items": [1, 2, 3],
                "names": ["Alice", "Bob"]
            }),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Object field should accept object with array values: {result:?}"
        );
    }

    // =========================================================================
    // NEGATIVE TESTS: Object field type rejects non-objects
    // =========================================================================

    #[test]
    fn test_object_field_rejects_array() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let entity = create_test_entity("data", json!(["item1", "item2"]), entity_def);

        let result = entity.validate();
        assert!(result.is_err(), "Object field should reject array");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("must be an object"),
            "Error message should mention 'must be an object', got: {error_msg}"
        );
    }

    #[test]
    fn test_object_field_rejects_array_of_objects() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let entity = create_test_entity(
            "data",
            json!([
                {"entity_type": "Customer", "count": 100}
            ]),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_err(),
            "Object field should reject array of objects"
        );
    }

    #[test]
    fn test_object_field_rejects_string() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let entity = create_test_entity("data", json!("not an object"), entity_def);

        let result = entity.validate();
        assert!(result.is_err(), "Object field should reject string");
    }

    #[test]
    fn test_object_field_rejects_number() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let entity = create_test_entity("data", json!(42), entity_def);

        let result = entity.validate();
        assert!(result.is_err(), "Object field should reject number");
    }

    #[test]
    fn test_object_field_rejects_boolean() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let entity = create_test_entity("data", json!(true), entity_def);

        let result = entity.validate();
        assert!(result.is_err(), "Object field should reject boolean");
    }

    // =========================================================================
    // POSITIVE TESTS: Array field type accepts JSON arrays
    // =========================================================================

    #[test]
    fn test_array_field_accepts_array() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity("data", json!(["item1", "item2"]), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Array field should accept array: {result:?}"
        );
    }

    #[test]
    fn test_array_field_accepts_empty_array() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity("data", json!([]), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Array field should accept empty array: {result:?}"
        );
    }

    #[test]
    fn test_array_field_accepts_array_of_objects() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity(
            "data",
            json!([
                {"entity_type": "Customer", "count": 100},
                {"entity_type": "Product", "count": 50}
            ]),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Array field should accept array of objects: {result:?}"
        );
    }

    #[test]
    fn test_array_field_accepts_array_of_numbers() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity("data", json!([1, 2, 3, 4, 5]), entity_def);

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Array field should accept array of numbers: {result:?}"
        );
    }

    #[test]
    fn test_array_field_accepts_mixed_array() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity(
            "data",
            json!(["string", 42, true, null, {"key": "value"}]),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "Array field should accept mixed array: {result:?}"
        );
    }

    // =========================================================================
    // NEGATIVE TESTS: Array field type rejects non-arrays
    // =========================================================================

    #[test]
    fn test_array_field_rejects_object() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity("data", json!({"key": "value"}), entity_def);

        let result = entity.validate();
        assert!(result.is_err(), "Array field should reject object");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("must be an array"),
            "Error message should mention 'must be an array', got: {error_msg}"
        );
    }

    #[test]
    fn test_array_field_rejects_string() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity("data", json!("not an array"), entity_def);

        let result = entity.validate();
        assert!(result.is_err(), "Array field should reject string");
    }

    #[test]
    fn test_array_field_rejects_number() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity("data", json!(42), entity_def);

        let result = entity.validate();
        assert!(result.is_err(), "Array field should reject number");
    }

    #[test]
    fn test_array_field_rejects_boolean() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity("data", json!(false), entity_def);

        let result = entity.validate();
        assert!(result.is_err(), "Array field should reject boolean");
    }

    #[test]
    fn test_array_field_rejects_empty_object() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let entity = create_test_entity("data", json!({}), entity_def);

        let result = entity.validate();
        assert!(result.is_err(), "Array field should reject empty object");
    }

    // =========================================================================
    // REAL-WORLD SCENARIO TESTS: Statistics Submission Workflow
    // =========================================================================

    #[test]
    fn test_statistics_submission_cors_origins_with_json_type() {
        // This test simulates the statistics submission workflow where
        // cors_origins is a JSON array field
        let entity_def = create_entity_definition_with_field("cors_origins", FieldType::Json);
        let entity = create_test_entity(
            "cors_origins",
            json!(["https://example.com", "https://api.example.com"]),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "cors_origins (Json type) should accept array of strings: {result:?}"
        );
    }

    #[test]
    fn test_statistics_submission_entities_per_definition_with_json_type() {
        // This test simulates the entities_per_definition field which is an array of objects
        let entity_def =
            create_entity_definition_with_field("entities_per_definition", FieldType::Json);
        let entity = create_test_entity(
            "entities_per_definition",
            json!([
                {"entity_type": "Customer", "count": 100},
                {"entity_type": "Product", "count": 250},
                {"entity_type": "Order", "count": 50}
            ]),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "entities_per_definition (Json type) should accept array of objects: {result:?}"
        );
    }

    #[test]
    fn test_statistics_submission_entity_definitions_with_json_type() {
        // This test simulates the entity_definitions field which is a JSON object
        let entity_def = create_entity_definition_with_field("entity_definitions", FieldType::Json);
        let entity = create_test_entity(
            "entity_definitions",
            json!({
                "count": 5,
                "names": ["Customer", "Product", "Order", "User", "Invoice"]
            }),
            entity_def,
        );

        let result = entity.validate();
        assert!(
            result.is_ok(),
            "entity_definitions (Json type) should accept object: {result:?}"
        );
    }

    // =========================================================================
    // FIELD TYPE SERIALIZATION/DESERIALIZATION TESTS
    // =========================================================================

    #[test]
    fn test_field_type_json_serializes_correctly() {
        let serialized = serde_json::to_string(&FieldType::Json).unwrap();
        assert_eq!(serialized, "\"Json\"");
    }

    #[test]
    fn test_field_type_object_serializes_correctly() {
        let serialized = serde_json::to_string(&FieldType::Object).unwrap();
        assert_eq!(serialized, "\"Object\"");
    }

    #[test]
    fn test_field_type_array_serializes_correctly() {
        let serialized = serde_json::to_string(&FieldType::Array).unwrap();
        assert_eq!(serialized, "\"Array\"");
    }

    #[test]
    fn test_field_type_json_deserializes_correctly() {
        let field_type: FieldType = serde_json::from_str("\"Json\"").unwrap();
        assert_eq!(field_type, FieldType::Json);
    }

    #[test]
    fn test_field_type_object_deserializes_correctly() {
        let field_type: FieldType = serde_json::from_str("\"Object\"").unwrap();
        assert_eq!(field_type, FieldType::Object);
    }

    #[test]
    fn test_field_type_array_deserializes_correctly() {
        let field_type: FieldType = serde_json::from_str("\"Array\"").unwrap();
        assert_eq!(field_type, FieldType::Array);
    }

    #[test]
    fn test_json_and_object_are_distinct_types() {
        // Ensure Json and Object are not confused
        assert_ne!(FieldType::Json, FieldType::Object);
        assert_ne!(
            serde_json::to_string(&FieldType::Json).unwrap(),
            serde_json::to_string(&FieldType::Object).unwrap()
        );
    }

    #[test]
    fn test_json_and_array_are_distinct_types() {
        // Ensure Json and Array are not confused
        assert_ne!(FieldType::Json, FieldType::Array);
        assert_ne!(
            serde_json::to_string(&FieldType::Json).unwrap(),
            serde_json::to_string(&FieldType::Array).unwrap()
        );
    }

    // =========================================================================
    // ENTITY DEFINITION ROUND-TRIP TESTS
    // =========================================================================

    #[test]
    fn test_entity_definition_with_json_field_serializes_correctly() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);
        let serialized = serde_json::to_string(&entity_def).unwrap();

        // Verify the field type is serialized as "Json"
        assert!(
            serialized.contains(r#""field_type":"Json""#),
            "Serialized entity definition should contain field_type 'Json', got: {serialized}"
        );
    }

    #[test]
    fn test_entity_definition_with_object_field_serializes_correctly() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);
        let serialized = serde_json::to_string(&entity_def).unwrap();

        // Verify the field type is serialized as "Object"
        assert!(
            serialized.contains(r#""field_type":"Object""#),
            "Serialized entity definition should contain field_type 'Object', got: {serialized}"
        );
    }

    #[test]
    fn test_entity_definition_with_array_field_serializes_correctly() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);
        let serialized = serde_json::to_string(&entity_def).unwrap();

        // Verify the field type is serialized as "Array"
        assert!(
            serialized.contains(r#""field_type":"Array""#),
            "Serialized entity definition should contain field_type 'Array', got: {serialized}"
        );
    }

    #[test]
    fn test_entity_definition_json_field_round_trip() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Json);

        // Serialize and deserialize
        let serialized = serde_json::to_string(&entity_def).unwrap();
        let deserialized: EntityDefinition = serde_json::from_str(&serialized).unwrap();

        // Verify the field type is preserved
        assert_eq!(deserialized.fields[0].field_type, FieldType::Json);
    }

    #[test]
    fn test_entity_definition_object_field_round_trip() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Object);

        // Serialize and deserialize
        let serialized = serde_json::to_string(&entity_def).unwrap();
        let deserialized: EntityDefinition = serde_json::from_str(&serialized).unwrap();

        // Verify the field type is preserved
        assert_eq!(deserialized.fields[0].field_type, FieldType::Object);
    }

    #[test]
    fn test_entity_definition_array_field_round_trip() {
        let entity_def = create_entity_definition_with_field("data", FieldType::Array);

        // Serialize and deserialize
        let serialized = serde_json::to_string(&entity_def).unwrap();
        let deserialized: EntityDefinition = serde_json::from_str(&serialized).unwrap();

        // Verify the field type is preserved
        assert_eq!(deserialized.fields[0].field_type, FieldType::Array);
    }
}
