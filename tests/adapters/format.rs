#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_workflow::data::adapters::format::csv::CsvFormatHandler;
use r_data_core_workflow::data::adapters::format::json::JsonFormatHandler;
use r_data_core_workflow::data::adapters::format::FormatHandler;
use serde_json::json;

#[test]
fn test_csv_format_handler_type() {
    let handler = CsvFormatHandler::new();
    assert_eq!(handler.format_type(), "csv");
}

#[test]
fn test_csv_parse_simple() {
    let handler = CsvFormatHandler::new();
    let data = b"name,age\nJohn,30\nJane,25";
    let options = json!({"has_header": true});

    let result = handler.parse(data, &options);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["name"], "John");
    assert_eq!(parsed[0]["age"], "30");
    assert_eq!(parsed[1]["name"], "Jane");
    assert_eq!(parsed[1]["age"], "25");
}

#[test]
fn test_csv_parse_no_header() {
    let handler = CsvFormatHandler::new();
    let data = b"John,30\nJane,25";
    let options = json!({"has_header": false});

    let result = handler.parse(data, &options);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.len(), 2);
    // Without header, fields are named field_0, field_1, etc.
    // Check that the first field exists and has the expected value
    let first_obj = parsed[0].as_object().unwrap();
    let values: Vec<&str> = first_obj.values().filter_map(|v| v.as_str()).collect();
    assert!(values.contains(&"John"));
    assert!(values.contains(&"30"));
    let second_obj = parsed[1].as_object().unwrap();
    let values2: Vec<&str> = second_obj.values().filter_map(|v| v.as_str()).collect();
    assert!(values2.contains(&"Jane"));
    assert!(values2.contains(&"25"));
}

#[test]
fn test_csv_parse_custom_delimiter() {
    let handler = CsvFormatHandler::new();
    let data = b"name|age\nJohn|30\nJane|25";
    let options = json!({"has_header": true, "delimiter": "|"});

    let result = handler.parse(data, &options);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["name"], "John");
    assert_eq!(parsed[0]["age"], "30");
}

#[test]
fn test_csv_serialize() {
    let handler = CsvFormatHandler::new();
    let data = vec![
        json!({"name": "John", "age": "30"}),
        json!({"name": "Jane", "age": "25"}),
    ];
    let options = json!({"has_header": true});

    let result = handler.serialize(&data, &options);
    assert!(result.is_ok());
    let bytes = result.unwrap();
    let csv_str = String::from_utf8_lossy(&bytes);
    // CSV should contain header and data (order may vary)
    assert!(csv_str.contains("name") || csv_str.contains("age"));
    assert!(csv_str.contains("John"));
    assert!(csv_str.contains("30"));
    assert!(csv_str.contains("Jane"));
    assert!(csv_str.contains("25"));
}

#[test]
fn test_csv_serialize_no_header() {
    let handler = CsvFormatHandler::new();
    let data = vec![
        json!({"name": "John", "age": "30"}),
        json!({"name": "Jane", "age": "25"}),
    ];
    let options = json!({"has_header": false});

    let result = handler.serialize(&data, &options);
    assert!(result.is_ok());
    let bytes = result.unwrap();
    let csv_str = String::from_utf8_lossy(&bytes);
    // No header should be present
    assert!(!csv_str.contains("name") || !csv_str.contains("age") || csv_str.lines().count() == 2);
    // Data should be present
    assert!(csv_str.contains("John"));
    assert!(csv_str.contains("30"));
    assert!(csv_str.contains("Jane"));
    assert!(csv_str.contains("25"));
}

#[test]
fn test_csv_validate_options() {
    let handler = CsvFormatHandler::new();

    // Valid options
    let options = json!({"has_header": true, "delimiter": ",", "quote": "\""});
    assert!(handler.validate_options(&options).is_ok());

    // Invalid delimiter (too long)
    let options = json!({"delimiter": ",,"});
    assert!(handler.validate_options(&options).is_err());

    // Invalid quote (too long)
    let options = json!({"quote": "\"\""});
    assert!(handler.validate_options(&options).is_err());
}

#[test]
fn test_json_format_handler_type() {
    let handler = JsonFormatHandler::new();
    assert_eq!(handler.format_type(), "json");
}

#[test]
fn test_json_parse_array() {
    let handler = JsonFormatHandler::new();
    let data = b"[{\"name\":\"John\",\"age\":30},{\"name\":\"Jane\",\"age\":25}]";
    let options = json!({});

    let result = handler.parse(data, &options);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["name"], "John");
    assert_eq!(parsed[0]["age"], 30);
    assert_eq!(parsed[1]["name"], "Jane");
    assert_eq!(parsed[1]["age"], 25);
}

#[test]
fn test_json_parse_ndjson() {
    let handler = JsonFormatHandler::new();
    let data = b"{\"name\":\"John\",\"age\":30}\n{\"name\":\"Jane\",\"age\":25}";
    let options = json!({"format": "ndjson"});

    let result = handler.parse(data, &options);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["name"], "John");
    assert_eq!(parsed[1]["name"], "Jane");
}

#[test]
fn test_json_serialize() {
    let handler = JsonFormatHandler::new();
    let data = vec![
        json!({"name": "John", "age": 30}),
        json!({"name": "Jane", "age": 25}),
    ];
    let options = json!({});

    let result = handler.serialize(&data, &options);
    assert!(result.is_ok());
    let bytes = result.unwrap();
    let json_str = String::from_utf8_lossy(&bytes);
    assert!(json_str.contains("John"));
    assert!(json_str.contains("Jane"));
}

#[test]
fn test_json_serialize_ndjson() {
    let handler = JsonFormatHandler::new();
    let data = vec![
        json!({"name": "John", "age": 30}),
        json!({"name": "Jane", "age": 25}),
    ];
    let options = json!({"as_array": false});

    let result = handler.serialize(&data, &options);
    assert!(result.is_ok());
    let bytes = result.unwrap();
    let json_str = String::from_utf8_lossy(&bytes);
    // NDJSON should have newlines between objects
    assert!(json_str.contains('\n'));
    assert!(json_str.contains("John"));
    assert!(json_str.contains("Jane"));
}

#[test]
fn test_json_validate_options() {
    let handler = JsonFormatHandler::new();

    // JSON format has minimal validation
    let options = json!({});
    assert!(handler.validate_options(&options).is_ok());

    let options = json!({"format": "ndjson"});
    assert!(handler.validate_options(&options).is_ok());
}
