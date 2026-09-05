use crate::dsl::to::{mapping_of, EntityWriteMode, ToDef};
use std::collections::HashMap;

fn mapping_with(key: &str, value: &str) -> HashMap<String, String> {
    HashMap::from([(key.to_string(), value.to_string())])
}

#[test]
fn mapping_of_format_variant() {
    let def = ToDef::Format {
        output: crate::dsl::to::OutputMode::Download,
        format: crate::dsl::from::FormatConfig {
            format_type: "json".to_string(),
            options: serde_json::json!({}),
        },
        mapping: mapping_with("dst", "src"),
    };
    assert_eq!(mapping_of(&def).get("dst"), Some(&"src".to_string()));
}

#[test]
fn mapping_of_entity_variant() {
    let def = ToDef::Entity {
        entity_definition: "user".to_string(),
        path: None,
        mode: EntityWriteMode::Create,
        identify: None,
        update_key: None,
        mapping: mapping_with("dst", "src"),
    };
    assert_eq!(mapping_of(&def).get("dst"), Some(&"src".to_string()));
}

#[test]
fn mapping_of_next_step_variant() {
    let def = ToDef::NextStep {
        mapping: mapping_with("dst", "src"),
    };
    assert_eq!(mapping_of(&def).get("dst"), Some(&"src".to_string()));
}

#[test]
fn mapping_of_email_variant() {
    let def = ToDef::Email {
        template_uuid: "uuid".to_string(),
        to: vec![],
        cc: None,
        mapping: mapping_with("dst", "src"),
    };
    assert_eq!(mapping_of(&def).get("dst"), Some(&"src".to_string()));
}
