#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use serde_json::json;

use crate::domain::dynamic_entity::entity::DynamicEntity;
use crate::entity_definition::definition::EntityDefinition;

fn create_test_entity() -> DynamicEntity {
    let definition = Arc::new(EntityDefinition::default());
    DynamicEntity::new("test_type".to_string(), definition)
}

mod read_only_fields {
    use super::*;

    #[test]
    fn test_set_uuid_first_time_succeeds() {
        let mut entity = create_test_entity();
        // uuid is not in field_data after new(), so first set should succeed
        assert!(!entity.field_data.contains_key("uuid"));
        assert!(entity
            .set("uuid", "550e8400-e29b-41d4-a716-446655440000".to_string())
            .is_ok());
        assert!(entity.field_data.contains_key("uuid"));
    }

    #[test]
    fn test_set_uuid_second_time_returns_read_only_error() {
        let mut entity = create_test_entity();
        entity
            .set("uuid", "550e8400-e29b-41d4-a716-446655440000".to_string())
            .unwrap();

        let result = entity.set("uuid", "00000000-0000-0000-0000-000000000000".to_string());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Read-only field"),
            "Expected ReadOnlyField error, got: {err}"
        );
        assert!(err.contains("uuid"));
    }

    #[test]
    fn test_created_at_already_set_returns_read_only_error() {
        let entity = create_test_entity();
        // created_at is set during new()
        assert!(entity.field_data.contains_key("created_at"));

        let mut entity = entity;
        let result = entity.set("created_at", "2024-01-01T00:00:00Z".to_string());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Read-only field"),
            "Expected ReadOnlyField error, got: {err}"
        );
        assert!(err.contains("created_at"));
    }

    #[test]
    fn test_created_at_on_empty_entity_succeeds() {
        let definition = Arc::new(EntityDefinition::default());
        let mut entity = DynamicEntity::from_data(
            "test_type".to_string(),
            std::collections::HashMap::new(),
            definition,
        );
        // created_at not present, so set should succeed
        assert!(entity
            .set("created_at", "2024-06-15T12:00:00Z".to_string())
            .is_ok());
    }
}

mod updated_at_auto_timestamp {
    use super::*;

    #[test]
    fn test_set_updated_at_auto_updates_timestamp() {
        let mut entity = create_test_entity();
        // The auto-timestamp should not use our supplied value
        entity
            .set("updated_at", "1999-01-01T00:00:00Z".to_string())
            .unwrap();

        let new_value = entity.field_data.get("updated_at").unwrap();
        // The value should be an auto-generated timestamp, not our supplied value
        assert_ne!(new_value, &json!("1999-01-01T00:00:00Z"));
    }
}

mod regular_fields {
    use super::*;

    #[test]
    fn test_set_custom_field_succeeds() {
        let mut entity = create_test_entity();
        assert!(entity.set("name", "Test Entity".to_string()).is_ok());
        assert_eq!(
            entity.field_data.get("name").unwrap(),
            &json!("Test Entity")
        );
    }

    #[test]
    fn test_set_custom_field_overwrites() {
        let mut entity = create_test_entity();
        entity.set("name", "First".to_string()).unwrap();
        entity.set("name", "Second".to_string()).unwrap();
        assert_eq!(entity.field_data.get("name").unwrap(), &json!("Second"));
    }

    #[test]
    fn test_set_published_field_succeeds() {
        let mut entity = create_test_entity();
        assert!(entity.set("published", true).is_ok());
        assert_eq!(entity.field_data.get("published").unwrap(), &json!(true));
    }

    #[test]
    fn test_set_version_field_succeeds() {
        let mut entity = create_test_entity();
        assert!(entity.set("version", 2i64).is_ok());
        assert_eq!(entity.field_data.get("version").unwrap(), &json!(2));
    }
}
