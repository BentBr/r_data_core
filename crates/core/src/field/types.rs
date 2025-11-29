use serde::{Deserialize, Serialize};
use std::fmt;

/// Field types supported in entity definitions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    #[must_use]
    pub const fn is_relation(&self) -> bool {
        matches!(self, Self::ManyToOne | Self::ManyToMany)
    }
}

// Implement Display for FieldType for better error messages
impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String => write!(f, "String"),
            Self::Text => write!(f, "Text"),
            Self::Wysiwyg => write!(f, "Wysiwyg"),
            Self::Integer => write!(f, "Integer"),
            Self::Float => write!(f, "Float"),
            Self::Boolean => write!(f, "Boolean"),
            Self::DateTime => write!(f, "DateTime"),
            Self::Date => write!(f, "Date"),
            Self::Object => write!(f, "Object"),
            Self::Array => write!(f, "Array"),
            Self::Uuid => write!(f, "Uuid"),
            Self::Json => write!(f, "Json"),
            Self::ManyToOne => write!(f, "ManyToOne"),
            Self::ManyToMany => write!(f, "ManyToMany"),
            Self::Select => write!(f, "Select"),
            Self::MultiSelect => write!(f, "MultiSelect"),
            Self::Image => write!(f, "Image"),
            Self::File => write!(f, "File"),
        }
    }
}

/// Return SQL type for a given field type
#[must_use]
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
                    return format!("VARCHAR({max_len})");
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
            enum_name.map_or_else(|| "TEXT".to_string(), |name| format!("{name}_enum"))
        }
        FieldType::MultiSelect => "TEXT[]".to_string(),
        FieldType::Image | FieldType::File => "TEXT".to_string(), // Store path or ID
        FieldType::Object | FieldType::Array | FieldType::Json => "JSONB".to_string(), // Complex types as JSON
        _ => "TEXT".to_string(), // Default for any other types
    }
}

// Check if a field type is valid and supported
#[must_use]
pub fn is_valid_field_type(field_type: &str) -> bool {
    matches!(
        field_type,
        "String"
            | "Text"
            | "Wysiwyg"
            | "Integer"
            | "Float"
            | "Boolean"
            | "DateTime"
            | "Date"
            | "Object"
            | "Array"
            | "Uuid"
            | "Json"
            | "ManyToOne"
            | "ManyToMany"
            | "Select"
            | "MultiSelect"
            | "Image"
            | "File"
    )
}
