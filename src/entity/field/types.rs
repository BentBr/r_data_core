use serde::{Deserialize, Serialize};
use std::fmt;

/// Field types supported in class definitions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    // String types with UI variants
    String,
    Text,
    Wysiwyg,

    // Numeric types
    Integer,
    Float,

    // Boolean type
    Boolean,

    // Date types
    DateTime,
    Date,

    // Complex data types
    Object,
    Array,
    Uuid,
    Json,

    // Relations
    ManyToOne,
    ManyToMany,

    // Select types
    Select,
    MultiSelect,

    // Asset types
    Image,
    File,
}

impl FieldType {
    /// Check if this field type is a relation
    pub fn is_relation(&self) -> bool {
        matches!(self, FieldType::ManyToOne | FieldType::ManyToMany)
    }
}

// Implement Display for FieldType for better error messages
impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::String => write!(f, "String"),
            FieldType::Text => write!(f, "Text"),
            FieldType::Wysiwyg => write!(f, "Wysiwyg"),
            FieldType::Integer => write!(f, "Integer"),
            FieldType::Float => write!(f, "Float"),
            FieldType::Boolean => write!(f, "Boolean"),
            FieldType::DateTime => write!(f, "DateTime"),
            FieldType::Date => write!(f, "Date"),
            FieldType::Object => write!(f, "Object"),
            FieldType::Array => write!(f, "Array"),
            FieldType::Uuid => write!(f, "Uuid"),
            FieldType::Json => write!(f, "Json"),
            FieldType::ManyToOne => write!(f, "ManyToOne"),
            FieldType::ManyToMany => write!(f, "ManyToMany"),
            FieldType::Select => write!(f, "Select"),
            FieldType::MultiSelect => write!(f, "MultiSelect"),
            FieldType::Image => write!(f, "Image"),
            FieldType::File => write!(f, "File"),
        }
    }
}

/// Return SQL type for a given field type
pub fn get_sql_type_for_field(
    field_type: &FieldType,
    max_length: Option<usize>,
    enum_name: Option<&str>,
) -> String {
    match field_type {
        FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
            // Use VARCHAR with length limit if specified
            if let Some(max_len) = max_length {
                if max_len <= 255 {
                    return format!("VARCHAR({})", max_len);
                }
            }
            "TEXT".to_string()
        }
        FieldType::Integer => "BIGINT".to_string(),
        FieldType::Float => "DOUBLE PRECISION".to_string(),
        FieldType::Boolean => "BOOLEAN".to_string(),
        FieldType::DateTime => "TIMESTAMP WITH TIME ZONE".to_string(),
        FieldType::Date => "DATE".to_string(),
        FieldType::Uuid => "UUID".to_string(),
        FieldType::Select => {
            // If this is an enum-backed select, use enum type
            if let Some(name) = enum_name {
                format!("{}_enum", name)
            } else {
                "TEXT".to_string()
            }
        }
        FieldType::MultiSelect => "TEXT[]".to_string(),
        FieldType::Image => "TEXT".to_string(), // Store path or ID
        FieldType::File => "TEXT".to_string(),  // Store path or ID
        FieldType::Object | FieldType::Array | FieldType::Json => "JSONB".to_string(), // Complex types as JSON
        _ => "TEXT".to_string(),                // Default for any other types
    }
}

// Check if a field type is valid and supported
pub fn is_valid_field_type(field_type: &str) -> bool {
    match field_type {
        "String" | "Text" | "Wysiwyg" | "Integer" | "Float" | "Boolean" |
        "DateTime" | "Date" | "Object" | "Array" | "Uuid" | "Json" |
        "ManyToOne" | "ManyToMany" | "Select" | "MultiSelect" | "Image" | "File" => true,
        _ => false,
    }
}
