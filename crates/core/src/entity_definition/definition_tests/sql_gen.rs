use super::create_test_entity_definition;
use crate::entity_definition::definition::*;

// ── unique index SQL (original tests) ────────────────────────────────────────

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

// ── table name helpers ────────────────────────────────────────────────────────

#[test]
fn test_get_table_name_prepends_entity_prefix_and_lowercases() {
    let def = EntityDefinition {
        entity_type: "Product".to_string(),
        ..EntityDefinition::default()
    };
    assert_eq!(def.get_table_name(), "entity_product");
}

#[test]
fn test_table_name_lowercases_entity_type() {
    let def = EntityDefinition {
        entity_type: "MyType".to_string(),
        ..EntityDefinition::default()
    };
    assert_eq!(def.table_name(), "mytype");
}

#[test]
fn test_generate_sql_schema_is_alias_for_generate_schema_sql() {
    let def = create_test_entity_definition();
    assert_eq!(def.generate_sql_schema(), def.generate_schema_sql());
}

// ── CREATE TABLE structure ────────────────────────────────────────────────────

#[test]
fn test_generate_schema_sql_contains_create_table() {
    let def = create_test_entity_definition();
    let sql = def.generate_schema_sql();
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS entity_test"));
    assert!(sql.contains("uuid UUID PRIMARY KEY"));
}

#[test]
fn test_generate_schema_sql_required_field_has_not_null() {
    let mut def = create_test_entity_definition();
    def.fields[0].required = true;
    let sql = def.generate_schema_sql();
    assert!(sql.contains("name TEXT NOT NULL"));
}

#[test]
fn test_generate_schema_sql_indexed_field_creates_index() {
    let mut def = create_test_entity_definition();
    def.fields[0].indexed = true;
    let sql = def.generate_schema_sql();
    assert!(sql.contains("CREATE INDEX IF NOT EXISTS idx_entity_test_name"));
}
