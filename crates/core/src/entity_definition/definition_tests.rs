#![allow(clippy::unwrap_used)]

use super::definition::*;
use super::schema::Schema;
use crate::field::ui::UiSettings;
use crate::field::{FieldDefinition, FieldType};
use uuid::Uuid;

fn create_test_entity_definition() -> EntityDefinition {
    EntityDefinition {
        uuid: Uuid::now_v7(),
        entity_type: "test".to_string(),
        display_name: "Test Entity".to_string(),
        description: None,
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![FieldDefinition {
            name: "name".to_string(),
            display_name: "Name".to_string(),
            field_type: FieldType::String,
            description: None,
            required: false,
            indexed: false,
            filterable: false,
            unique: false,
            default_value: None,
            validation: crate::field::options::FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: std::collections::HashMap::new(),
        }],
        schema: Schema::default(),
        created_at: time::OffsetDateTime::now_utc(),
        updated_at: time::OffsetDateTime::now_utc(),
        created_by: Uuid::nil(),
        updated_by: None,
        published: false,
        version: 1,
    }
}

#[test]
fn test_generate_schema_sql_includes_unique_index() {
    let mut def = create_test_entity_definition();
    def.fields[0].unique = true;

    let sql = def.generate_schema_sql();

    assert!(
        sql.contains("CREATE UNIQUE INDEX IF NOT EXISTS idx_entity_test_name_unique"),
        "SQL should contain unique index creation"
    );
    assert!(
        sql.contains("ON entity_test (name)"),
        "SQL should specify correct table and column"
    );
}

#[test]
fn test_generate_schema_sql_no_unique_index_when_false() {
    let def = create_test_entity_definition();
    // unique is false by default

    let sql = def.generate_schema_sql();

    assert!(
        !sql.contains("CREATE UNIQUE INDEX"),
        "SQL should not CREATE unique index when unique is false"
    );
    assert!(
        sql.contains("DROP INDEX IF EXISTS"),
        "SQL should DROP unique index when unique is false (to clean up)"
    );
}

#[test]
fn test_generate_schema_sql_no_unique_index_when_not_set() {
    let def = create_test_entity_definition();

    let sql = def.generate_schema_sql();

    assert!(
        !sql.contains("CREATE UNIQUE INDEX"),
        "SQL should not CREATE unique index when unique is not set"
    );
    assert!(
        sql.contains("DROP INDEX IF EXISTS"),
        "SQL should DROP unique index when unique is not set (to clean up)"
    );
}

#[test]
fn test_generate_schema_sql_unique_index_comment() {
    let mut def = create_test_entity_definition();
    def.fields[0].unique = true;

    let sql = def.generate_schema_sql();

    assert!(
        sql.contains("-- UNIQUE: Field unique constraint"),
        "SQL should contain unique constraint comment"
    );
}
