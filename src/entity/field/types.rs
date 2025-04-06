use serde::{Deserialize, Serialize};

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
    UUID,
    
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

/// Return SQL type for a given field type
pub fn get_sql_type_for_field(
    field_type: &FieldType, 
    max_length: Option<usize>, 
    enum_name: Option<&str>
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
        },
        FieldType::Integer => "BIGINT".to_string(),
        FieldType::Float => "DOUBLE PRECISION".to_string(),
        FieldType::Boolean => "BOOLEAN".to_string(), 
        FieldType::DateTime => "TIMESTAMP WITH TIME ZONE".to_string(),
        FieldType::Date => "DATE".to_string(),
        FieldType::UUID => "UUID".to_string(),
        FieldType::Select => {
            // If this is an enum-backed select, use enum type
            if let Some(name) = enum_name {
                format!("{}_enum", name)
            } else {
                "TEXT".to_string()
            }
        },
        FieldType::MultiSelect => "TEXT[]".to_string(),
        FieldType::Image => "TEXT".to_string(), // Store path or ID
        FieldType::File => "TEXT".to_string(),  // Store path or ID
        FieldType::Object | FieldType::Array => "JSONB".to_string(), // Complex types as JSON
        _ => "TEXT".to_string(), // Default for any other types
    }
} 