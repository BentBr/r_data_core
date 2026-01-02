#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use log::warn;
use r_data_core_core::error::Result;
use serde_json::Value;
use std::collections::HashMap;

/// Apply transformation to a value based on transform type string
///
/// # Arguments
/// * `value` - Value to transform
/// * `transform_type` - Transform type ("lowercase", "uppercase", "trim", "normalize", "slug", "hash")
///
/// # Returns
/// Transformed value
#[must_use]
pub fn apply_value_transform(value: &Value, transform_type: &str) -> Value {
    let string_value = match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => return Value::Null,
        _ => value.to_string(),
    };

    let transformed = match transform_type {
        "lowercase" => string_value.to_lowercase(),
        "uppercase" => string_value.to_uppercase(),
        "trim" => string_value.trim().to_string(),
        "normalize" => {
            // Remove special characters, keep alphanumeric and common separators
            string_value
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                        c
                    } else {
                        ' '
                    }
                })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        }
        "slug" => {
            // Convert to URL-friendly slug
            string_value
                .to_lowercase()
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .replace("--", "-")
                .trim_matches('-')
                .to_string()
        }
        "hash" => {
            // Simple hash (for privacy-sensitive lookups)
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            string_value.hash(&mut hasher);
            format!("{:x}", hasher.finish())
        }
        _ => {
            warn!("Unknown transform type: {transform_type}, returning original value");
            string_value
        }
    };

    Value::String(transformed)
}

/// Apply transformations to filter values
///
/// # Arguments
/// * `filters` - Original filters
/// * `value_transforms` - Optional map of field names to transform type strings
///
/// # Returns
/// Transformed filters
#[must_use]
pub fn apply_filters_transforms<S: std::hash::BuildHasher>(
    filters: &HashMap<String, Value, S>,
    value_transforms: Option<&HashMap<String, String, S>>,
) -> HashMap<String, Value> {
    let mut transformed_filters = HashMap::new();
    for (field, value) in filters {
        let transformed_value = value_transforms.map_or_else(
            || value.clone(),
            |transforms| {
                transforms.get(field).map_or_else(
                    || value.clone(),
                    |transform_type| apply_value_transform(value, transform_type),
                )
            },
        );
        transformed_filters.insert(field.clone(), transformed_value);
    }
    transformed_filters
}

/// Build a path by concatenating values from input fields
///
/// # Arguments
/// * `path_template` - Template with placeholders (e.g., "/{field1}/{field2}")
/// * `input` - Input JSON/CSV data
/// * `separator` - Separator between path segments (default: "/")
/// * `field_transforms` - Optional map of field names to transform type strings
///
/// # Returns
/// Built path string
///
/// # Errors
/// Returns an error if template parsing fails or required fields are missing
pub fn build_path_from_fields<S: std::hash::BuildHasher>(
    path_template: &str,
    input: &Value,
    separator: Option<&str>,
    field_transforms: Option<&HashMap<String, String, S>>,
) -> Result<String> {
    let sep = separator.unwrap_or("/");
    let mut result = String::new();
    let mut current_pos = 0;

    // Find placeholders in template (e.g., {field_name})
    let template_chars: Vec<char> = path_template.chars().collect();
    let mut i = 0;

    while i < template_chars.len() {
        if template_chars[i] == '{' {
            // Find closing brace
            let start = i + 1;
            let mut end = start;
            while end < template_chars.len() && template_chars[end] != '}' {
                end += 1;
            }

            if end < template_chars.len() {
                // Extract field name
                let field_name: String = template_chars[start..end].iter().collect();

                // Add text before placeholder
                if current_pos < start - 1 {
                    let text: String = template_chars[current_pos..start - 1].iter().collect();
                    result.push_str(&text);
                }

                // Get field value from input
                let field_value = input.get(&field_name).ok_or_else(|| {
                    r_data_core_core::error::Error::Validation(format!(
                        "Field '{field_name}' not found in input for path template"
                    ))
                })?;

                // Apply transformation if specified
                let transformed_value = field_transforms.map_or_else(
                    || field_value.clone(),
                    |transforms| {
                        transforms.get(&field_name).map_or_else(
                            || field_value.clone(),
                            |transform_type| apply_value_transform(field_value, transform_type),
                        )
                    },
                );

                // Convert to string
                let value_str = match transformed_value {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => {
                        return Err(r_data_core_core::error::Error::Validation(format!(
                            "Field '{field_name}' is null, cannot build path"
                        )));
                    }
                    _ => transformed_value.to_string(),
                };

                // Add separator if not at start and result is not empty
                if !result.is_empty() && !result.ends_with(sep) {
                    result.push_str(sep);
                }

                result.push_str(&value_str);

                // Move past closing brace
                i = end + 1;
                current_pos = i;
            } else {
                // No closing brace found, treat as literal
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    // Add remaining text after last placeholder
    if current_pos < template_chars.len() {
        let text: String = template_chars[current_pos..].iter().collect();
        if !text.is_empty() {
            if !result.is_empty() && !result.ends_with(sep) {
                result.push_str(sep);
            }
            result.push_str(&text);
        }
    }

    // Normalize path (remove double separators, ensure starts with /)
    let normalized = if result.starts_with(sep) {
        result
    } else {
        format!("{sep}{result}")
    };

    Ok(normalized.replace(&format!("{sep}{sep}"), sep))
}

/// Parse path to extract entity key and parent path
///
/// # Arguments
/// * `path` - Full path to the entity
///
/// # Returns
/// (`normalized_path`, `entity_key`, `parent_path_opt`)
#[must_use]
pub fn parse_entity_path(path: &str) -> (String, String, Option<String>) {
    // Normalize path
    let normalized_path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };

    // Parse path to determine parent and entity_key
    let path_parts: Vec<&str> = normalized_path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    if path_parts.is_empty() {
        return (normalized_path, String::new(), None);
    }

    let entity_key = path_parts[path_parts.len() - 1].to_string();

    let parent_path = if path_parts.len() > 1 {
        Some(format!(
            "/{}",
            path_parts[0..path_parts.len() - 1].join("/")
        ))
    } else {
        None
    };

    (normalized_path, entity_key, parent_path)
}
