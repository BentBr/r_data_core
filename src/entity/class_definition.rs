use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sqlx::PgPool;
use log;
use regex::Regex;
use chrono;
use uuid::Uuid;
use std::sync::Arc;

use crate::db::migrations;
use super::AbstractRDataEntity;
use super::field::{FieldType, FieldDefinition, FieldValidation, UiSettings, OptionsSource, SelectOption};
use super::value::{FromValue, ToValue, Value};
use crate::error::{Error, Result};

// Re-export field types for convenience
pub use super::field::{FieldType, FieldDefinition, FieldValidation, UiSettings, OptionsSource, SelectOption};

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

/// Definition of a field in a class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Field name (must be unique within class)
    pub name: String,
    
    /// User-friendly display name
    pub display_name: String,
    
    /// Field data type
    pub field_type: FieldType,
    
    /// Field description for admin UI
    pub description: Option<String>,
    
    /// Whether the field is required
    pub required: bool,
    
    /// Whether the field is indexed for faster searches
    pub indexed: bool,
    
    /// Whether the field can be used in API filtering
    pub filterable: bool,
    
    /// Default value for the field as JSON
    pub default_value: Option<serde_json::Value>,
    
    /// Field validation/constraints
    #[serde(default)]
    pub validation: FieldValidation,
    
    /// UI settings for the field
    #[serde(default)]
    pub ui_settings: UiSettings,
    
    /// Extra field constraints or validation rules
    pub constraints: HashMap<String, serde_json::Value>,
}

/// Validation rules for fields
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldValidation {
    /// Minimum string length
    pub min_length: Option<usize>,
    
    /// Maximum string length 
    pub max_length: Option<usize>,
    
    /// Regex pattern for string validation
    pub pattern: Option<String>,
    
    /// Minimum numeric value
    pub min_value: Option<serde_json::Value>,
    
    /// Maximum numeric value
    pub max_value: Option<serde_json::Value>,
    
    /// Allow only positive values for numeric fields
    pub positive_only: Option<bool>,
    
    /// Minimum date (ISO string or "now")
    pub min_date: Option<String>,
    
    /// Maximum date (ISO string or "now")
    pub max_date: Option<String>,
    
    /// For relation fields: target entity class
    pub target_class: Option<String>,
    
    /// For select fields: options source
    pub options_source: Option<OptionsSource>,
}

/// Source of options for select fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum OptionsSource {
    /// Fixed list of options
    #[serde(rename = "fixed")]
    Fixed {
        options: Vec<SelectOption>
    },
    
    /// Options from an enum stored in database
    #[serde(rename = "enum")]
    Enum {
        enum_name: String
    },
    
    /// Options from a database query
    #[serde(rename = "query")]
    Query {
        entity_type: String,
        value_field: String,
        label_field: String,
        filter: Option<serde_json::Value>
    }
}

/// Option for select fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// UI settings for fields
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiSettings {
    /// Placeholder text for input fields
    pub placeholder: Option<String>,
    
    /// Help text to display in UI
    pub help_text: Option<String>,
    
    /// Whether to hide this field in list views
    pub hide_in_lists: Option<bool>,
    
    /// Layout width (1-12 for 12-column grid)
    pub width: Option<u8>,
    
    /// Order in form (lower numbers appear first)
    pub order: Option<i32>,
    
    /// Group name for visually grouping fields
    pub group: Option<String>,
    
    /// CSS class names to apply to field
    pub css_class: Option<String>,
    
    /// For WYSIWYG: toolbar configuration
    pub wysiwyg_toolbar: Option<String>,
    
    /// For input fields: input type (e.g., "password", "email")
    pub input_type: Option<String>,
}

/// Class definition for a custom RDataEntity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDefinition {
    /// Base entity properties
    #[serde(flatten)]
    pub base: AbstractRDataEntity,
    
    /// Unique class name (used in code/API)
    pub class_name: String,
    
    /// Display name for admin UI
    pub display_name: String,
    
    /// Class description
    pub description: Option<String>,
    
    /// Group name for organizing classes
    pub group: Option<String>,
    
    /// Whether instances can have children
    pub allow_children: bool,
    
    /// Icon for admin UI
    pub icon: Option<String>,
    
    /// Field definitions for this class
    pub fields: Vec<FieldDefinition>,
}

impl ClassDefinition {
    /// Create a new class definition
    pub fn new(class_name: String, display_name: String) -> Self {
        Self {
            base: AbstractRDataEntity::new("/classes".to_string()),
            class_name,
            display_name,
            description: None,
            group: None,
            allow_children: false,
            icon: None,
            fields: Vec::new(),
        }
    }
    
    /// Add a field to the class definition
    pub fn add_field(&mut self, field: FieldDefinition) -> Result<(), String> {
        // Check if field with same name already exists
        if self.fields.iter().any(|f| f.name == field.name) {
            return Err(format!("Field with name '{}' already exists", field.name));
        }
        
        self.fields.push(field);
        Ok(())
    }
    
    /// Get a field definition by name
    pub fn get_field(&self, name: &str) -> Option<&FieldDefinition> {
        self.fields.iter().find(|f| f.name == name)
    }
    
    /// Generate the SQL table name for this entity type
    pub fn get_table_name(&self) -> String {
        format!("entity_{}", self.class_name.to_lowercase())
    }
    
    /// Generate SQL table schema for this class
    pub fn generate_sql_schema(&self) -> String {
        let table_name = self.get_table_name();
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table_name);
        sql.push_str("  id BIGSERIAL PRIMARY KEY,\n");
        sql.push_str("  uuid UUID NOT NULL,\n");
        sql.push_str("  path TEXT NOT NULL,\n");
        sql.push_str("  created_at TIMESTAMP WITH TIME ZONE NOT NULL,\n");
        sql.push_str("  updated_at TIMESTAMP WITH TIME ZONE NOT NULL,\n");
        sql.push_str("  created_by BIGINT,\n");
        sql.push_str("  updated_by BIGINT,\n");
        sql.push_str("  published BOOLEAN NOT NULL DEFAULT FALSE,\n");
        sql.push_str("  version INTEGER NOT NULL DEFAULT 1,\n");
        
        // Add custom fields that should be columns
        for field in &self.fields {
            // Skip relation fields as they'll be in relation tables
            if matches!(field.field_type, FieldType::ManyToOne | FieldType::ManyToMany) {
                // For ManyToOne, we do need a reference column in this table
                if matches!(field.field_type, FieldType::ManyToOne) {
                    if let Some(target_class) = &field.validation.target_class {
                        sql.push_str(&format!("  {}_id BIGINT,\n", field.name));
                    }
                }
                continue;
            }
            
            let sql_type = match field.field_type {
                FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
                    // Use VARCHAR with length limit if specified
                    if let Some(max_length) = field.validation.max_length {
                        if max_length <= 255 {
                            format!("VARCHAR({})", max_length)
                        } else {
                            "TEXT".to_string()
                        }
                    } else {
                        "TEXT".to_string()
                    }
                },
                FieldType::Integer => "BIGINT",
                FieldType::Float => "DOUBLE PRECISION",
                FieldType::Boolean => "BOOLEAN", 
                FieldType::DateTime => "TIMESTAMP WITH TIME ZONE",
                FieldType::Date => "DATE",
                FieldType::UUID => "UUID",
                FieldType::Select => {
                    // If this is an enum-backed select, use enum type
                    if let Some(OptionsSource::Enum { enum_name }) = &field.validation.options_source {
                        format!("{}_enum", enum_name)
                    } else {
                        "TEXT".to_string()
                    }
                },
                FieldType::MultiSelect => "TEXT[]",
                FieldType::Image => "TEXT", // Store path or ID
                FieldType::File => "TEXT",  // Store path or ID
                FieldType::Object | FieldType::Array => "JSONB", // Complex types as JSON
                _ => continue, // Skip other types
            };
            
            let null_constraint = if field.required { "NOT NULL" } else { "" };
            
            // Add constraints if applicable
            let mut constraints = String::new();
            
            // Add numeric constraints
            if matches!(field.field_type, FieldType::Integer | FieldType::Float) {
                // Add CHECK constraints for min/max/positive
                let mut checks = Vec::new();
                
                if let Some(min) = &field.validation.min_value {
                    if let Some(min_num) = min.as_i64().or_else(|| min.as_f64().map(|f| f as i64)) {
                        checks.push(format!("{} >= {}", field.name, min_num));
                    }
                }
                
                if let Some(max) = &field.validation.max_value {
                    if let Some(max_num) = max.as_i64().or_else(|| max.as_f64().map(|f| f as i64)) {
                        checks.push(format!("{} <= {}", field.name, max_num));
                    }
                }
                
                if let Some(true) = field.validation.positive_only {
                    checks.push(format!("{} >= 0", field.name));
                }
                
                if !checks.is_empty() {
                    constraints = format!(" CHECK ({})", checks.join(" AND "));
                }
            }
            
            sql.push_str(&format!("  {} {}{}{},\n", field.name, sql_type, null_constraint, constraints));
        }
        
        // Add custom_fields JSONB for any additional fields not in schema
        sql.push_str("  custom_fields JSONB NOT NULL DEFAULT '{}'\n");
        sql.push_str(");\n");
        
        // Add indexes for searchable fields
        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_uuid ON {} (uuid);\n", 
            table_name, table_name));
            
        for field in &self.fields {
            if field.indexed && !matches!(field.field_type, FieldType::ManyToMany) {
                // For ManyToOne fields, index the foreign key
                if matches!(field.field_type, FieldType::ManyToOne) {
                    if field.validation.target_class.is_some() {
                        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{}_id ON {} ({}_id);\n", 
                            table_name, field.name, table_name, field.name));
                    }
                } else if !matches!(field.field_type, FieldType::Object | FieldType::Array) {
                    sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_{} ON {} ({});\n", 
                        table_name, field.name, table_name, field.name));
                }
            }
        }
        
        // Generate relation tables
        for field in &self.fields {
            if matches!(field.field_type, FieldType::ManyToOne | FieldType::ManyToMany) {
                if let Some(target_class) = &field.validation.target_class {
                    let target_table = format!("entity_{}", target_class.to_lowercase());
                    
                    // For ManyToOne, add foreign key constraint
                    if matches!(field.field_type, FieldType::ManyToOne) {
                        sql.push_str(&format!(
                            "ALTER TABLE {} ADD CONSTRAINT fk_{}_{} FOREIGN KEY ({}_id) REFERENCES {} (id) ON DELETE SET NULL;\n",
                            table_name, table_name, field.name, field.name, target_table
                        ));
                    }
                    
                    // For ManyToMany, create a join table
                    if matches!(field.field_type, FieldType::ManyToMany) {
                        let relation_table = format!("{}_{}_{}_relation", 
                            self.class_name.to_lowercase(), 
                            field.name,
                            target_class.to_lowercase());
                            
                        sql.push_str(&format!("CREATE TABLE IF NOT EXISTS {} (\n", relation_table));
                        sql.push_str("  id BIGSERIAL PRIMARY KEY,\n");
                        
                        // Reference to this entity
                        sql.push_str(&format!("  {}_id BIGINT NOT NULL REFERENCES {} (id) ON DELETE CASCADE,\n", 
                            self.class_name.to_lowercase(), table_name));
                            
                        // Reference to target entity
                        sql.push_str(&format!("  {}_id BIGINT NOT NULL REFERENCES {} (id) ON DELETE CASCADE,\n", 
                            target_class.to_lowercase(), target_table));
                            
                        // Add position field for ordered relations and metadata
                        sql.push_str("  position INTEGER NOT NULL DEFAULT 0,\n");
                        sql.push_str("  metadata JSONB,\n");
                        
                        // Add unique constraint to prevent duplicates
                        sql.push_str(&format!("  UNIQUE({}_id, {}_id)\n", 
                            self.class_name.to_lowercase(), target_class.to_lowercase()));
                        sql.push_str(");\n");
                        
                        // Add indices for faster lookups
                        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_source ON {} ({}_id);\n", 
                            relation_table, relation_table, self.class_name.to_lowercase()));
                            
                        sql.push_str(&format!("CREATE INDEX IF NOT EXISTS idx_{}_target ON {} ({}_id);\n", 
                            relation_table, relation_table, target_class.to_lowercase()));
                    }
                }
            }
        }
        
        sql
    }
    
    /// Apply the class definition to the database 
    pub async fn apply_to_database(&self, db: &PgPool) -> Result<(), Error> {
        // 1. Generate SQL schema and execute
        let sql = self.generate_sql_schema();
        debug!("Applying SQL schema for class {}: {}", self.class_name, sql);
        
        // Execute the SQL schema
        sqlx::query(&sql)
            .execute(db)
            .await
            .map_err(Error::Database)?;
        
        // Create enum types for select fields if needed
        for field in &self.fields {
            if matches!(field.field_type, FieldType::Select | FieldType::MultiSelect) {
                if let Some(options_source) = &field.validation.options_source {
                    match options_source {
                        OptionsSource::Enum { enum_name } => {
                            // For enum type, we need to extract values from somewhere
                            // If no fixed options provided, we create an empty enum that can be populated later
                            let values = Vec::new();
                            crate::db::migrations::create_or_update_enum(db, enum_name, &values).await?;
                        },
                        OptionsSource::Fixed { options } => {
                            // For fixed options, create an enum with the option values
                            // Only if the field needs an enum - not all select fields do
                            if field.constraints.get("use_enum").map_or(false, |v| v.as_bool().unwrap_or(false)) {
                                let enum_name = format!("{}_{}", self.class_name.to_lowercase(), field.name.to_lowercase());
                                let values: Vec<String> = options.iter().map(|opt| opt.value.clone()).collect();
                                crate::db::migrations::create_or_update_enum(db, &enum_name, &values).await?;
                            }
                        },
                        // Query options don't need enum types
                        OptionsSource::Query { .. } => {}
                    }
                }
            }
        }
        
        // Register in entities registry if not exists
        let entity_exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM entity_registry WHERE class_name = $1)",
        )
        .bind(&self.class_name)
        .fetch_one(db)
        .await
        .map_err(Error::Database)?;
        
        // Insert or update the entity in the registry
        if entity_exists.0 {
            sqlx::query(
                "UPDATE entity_registry SET display_name = $2 WHERE class_name = $1",
            )
            .bind(&self.class_name)
            .bind(&self.display_name)
            .execute(db)
            .await
            .map_err(Error::Database)?;
        } else {
            sqlx::query(
                "INSERT INTO entity_registry (class_name, display_name) VALUES ($1, $2)",
            )
            .bind(&self.class_name)
            .bind(&self.display_name)
            .execute(db)
            .await
            .map_err(Error::Database)?;
        }
        
        Ok(())
    }
}

pub struct DynamicEntity {
    entity_type: String,
    data: HashMap<String, Value>,
    definition: Arc<ClassDefinition>,
}

impl DynamicEntity {
    pub fn get<T: FromValue>(&self, field: &str) -> Option<T> {
        self.data.get(field).and_then(|v| T::from_value(v))
    }
    
    pub fn set<T: ToValue>(&mut self, field: &str, value: T) -> Result<(), Error> {
        // Validate against field definition
        if let Some(field_def) = self.definition.get_field(field) {
            // Type validation...
            self.data.insert(field.to_string(), value.to_value());
            Ok(())
        } else {
            Err(Error::Validation(format!("Unknown field: {}", field)))
        }
    }
}

// Typed wrapper
pub struct Product(DynamicEntity);

impl Product {
    // Type-safe accessors
    pub fn name(&self) -> Option<String> {
        self.0.get("name")
    }
    
    pub fn set_name(&mut self, name: String) -> Result<(), Error> {
        self.0.set("name", name)
    }
    
    // Generated dynamically based on class definition
}

impl FieldDefinition {
    /// Validate a value against this field's type and constraints
    pub fn validate_value(&self, value: &serde_json::Value) -> Result<(), String> {
        // For null values, only check if field is required
        if value.is_null() {
            if self.required {
                return Err(format!("Field '{}' is required", self.name));
            } else {
                return Ok(()); // Null is valid for optional fields
            }
        }

        // Type validation
        match self.field_type {
            FieldType::String | FieldType::Text | FieldType::Wysiwyg => {
                // String type validation
                if !value.is_string() {
                    return Err(format!("Field '{}' must be a string", self.name));
                }
                
                let string_value = value.as_str().unwrap();
                
                // Length validation
                if let Some(min_length) = self.validation.min_length {
                    if string_value.len() < min_length {
                        return Err(format!("Field '{}' must be at least {} characters", self.name, min_length));
                    }
                }
                
                if let Some(max_length) = self.validation.max_length {
                    if string_value.len() > max_length {
                        return Err(format!("Field '{}' must be no more than {} characters", self.name, max_length));
                    }
                }
                
                // Pattern validation
                if let Some(pattern) = &self.validation.pattern {
                    match Regex::new(pattern) {
                        Ok(re) => {
                            if !re.is_match(string_value) {
                                return Err(format!("Field '{}' must match pattern: {}", self.name, pattern));
                            }
                        },
                        Err(_) => {
                            return Err(format!("Invalid regex pattern for field '{}': {}", self.name, pattern));
                        }
                    }
                }
            },
            
            FieldType::Integer => {
                // Integer type validation
                if !value.is_i64() && !value.is_u64() && !(value.is_string() && value.as_str().unwrap().parse::<i64>().is_ok()) {
                    return Err(format!("Field '{}' must be an integer", self.name));
                }
                
                let num_value = if value.is_i64() {
                    value.as_i64().unwrap()
                } else if value.is_u64() {
                    value.as_u64().unwrap() as i64
                } else {
                    value.as_str().unwrap().parse::<i64>().unwrap()
                };
                
                // Range validation
                if let Some(min_value) = &self.validation.min_value {
                    if let Some(min) = min_value.as_i64() {
                        if num_value < min {
                            return Err(format!("Field '{}' must be at least {}", self.name, min));
                        }
                    }
                }
                
                if let Some(max_value) = &self.validation.max_value {
                    if let Some(max) = max_value.as_i64() {
                        if num_value > max {
                            return Err(format!("Field '{}' must be no more than {}", self.name, max));
                        }
                    }
                }
                
                // Positive only validation
                if let Some(true) = self.validation.positive_only {
                    if num_value < 0 {
                        return Err(format!("Field '{}' must be a positive number", self.name));
                    }
                }
            },
            
            FieldType::Float => {
                // Float type validation
                if !value.is_number() && !(value.is_string() && value.as_str().unwrap().parse::<f64>().is_ok()) {
                    return Err(format!("Field '{}' must be a number", self.name));
                }
                
                let num_value = if value.is_f64() {
                    value.as_f64().unwrap()
                } else if value.is_i64() {
                    value.as_i64().unwrap() as f64
                } else if value.is_u64() {
                    value.as_u64().unwrap() as f64
                } else {
                    value.as_str().unwrap().parse::<f64>().unwrap()
                };
                
                // Range validation
                if let Some(min_value) = &self.validation.min_value {
                    if let Some(min) = min_value.as_f64() {
                        if num_value < min {
                            return Err(format!("Field '{}' must be at least {}", self.name, min));
                        }
                    }
                }
                
                if let Some(max_value) = &self.validation.max_value {
                    if let Some(max) = max_value.as_f64() {
                        if num_value > max {
                            return Err(format!("Field '{}' must be no more than {}", self.name, max));
                        }
                    }
                }
                
                // Positive only validation
                if let Some(true) = self.validation.positive_only {
                    if num_value < 0.0 {
                        return Err(format!("Field '{}' must be a positive number", self.name));
                    }
                }
            },
            
            FieldType::Boolean => {
                // Boolean type validation
                if !value.is_boolean() && 
                   !(value.is_string() && ["true", "false", "yes", "no", "1", "0"].contains(&value.as_str().unwrap().to_lowercase().as_str())) {
                    return Err(format!("Field '{}' must be a boolean", self.name));
                }
            },
            
            FieldType::Date | FieldType::DateTime => {
                // Date/DateTime type validation
                if !value.is_string() {
                    return Err(format!("Field '{}' must be a date string", self.name));
                }
                
                let date_str = value.as_str().unwrap();
                
                // Validate ISO date format
                let parsed_date = if self.field_type == FieldType::DateTime {
                    chrono::DateTime::parse_from_rfc3339(date_str)
                } else {
                    // For Date type, try parsing as YYYY-MM-DD
                    let naive_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
                    naive_date.map(|d| {
                        chrono::DateTime::from_utc(
                            chrono::NaiveDateTime::new(d, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
                            chrono::Utc
                        )
                    })
                };
                
                if let Err(_) = parsed_date {
                    return Err(format!("Field '{}' must be a valid date/time format", self.name));
                }
                
                let parsed_dt = parsed_date.unwrap();
                
                // Min date validation
                if let Some(min_date_str) = &self.validation.min_date {
                    let min_date = if min_date_str == "now" {
                        chrono::Utc::now()
                    } else {
                        match chrono::DateTime::parse_from_rfc3339(min_date_str) {
                            Ok(dt) => dt.into(),
                            Err(_) => {
                                return Err(format!("Invalid min_date format for field '{}'", self.name));
                            }
                        }
                    };
                    
                    if parsed_dt < min_date {
                        return Err(format!("Field '{}' date must be after {}", self.name, min_date.to_rfc3339()));
                    }
                }
                
                // Max date validation
                if let Some(max_date_str) = &self.validation.max_date {
                    let max_date = if max_date_str == "now" {
                        chrono::Utc::now()
                    } else {
                        match chrono::DateTime::parse_from_rfc3339(max_date_str) {
                            Ok(dt) => dt.into(),
                            Err(_) => {
                                return Err(format!("Invalid max_date format for field '{}'", self.name));
                            }
                        }
                    };
                    
                    if parsed_dt > max_date {
                        return Err(format!("Field '{}' date must be before {}", self.name, max_date.to_rfc3339()));
                    }
                }
            },
            
            FieldType::UUID => {
                // UUID type validation
                if !value.is_string() {
                    return Err(format!("Field '{}' must be a UUID string", self.name));
                }
                
                let uuid_str = value.as_str().unwrap();
                if let Err(_) = uuid::Uuid::parse_str(uuid_str) {
                    return Err(format!("Field '{}' must be a valid UUID", self.name));
                }
            },
            
            FieldType::Select => {
                // Select type validation
                if !value.is_string() {
                    return Err(format!("Field '{}' must be a string value", self.name));
                }
                
                let select_value = value.as_str().unwrap();
                
                // Validate against options source if available
                if let Some(ref options_source) = self.validation.options_source {
                    match options_source {
                        OptionsSource::Fixed { options } => {
                            let valid_values: Vec<&str> = options.iter().map(|opt| opt.value.as_str()).collect();
                            if !valid_values.contains(&select_value) {
                                return Err(format!("Field '{}' must be one of: {}", self.name, valid_values.join(", ")));
                            }
                        },
                        // For Enum and Query types, validation happens at DB level
                        _ => {}
                    }
                }
            },
            
            FieldType::MultiSelect => {
                // MultiSelect type validation - should be an array of strings
                if !value.is_array() {
                    return Err(format!("Field '{}' must be an array of values", self.name));
                }
                
                let array = value.as_array().unwrap();
                
                // Check that all array elements are strings
                if !array.iter().all(|v| v.is_string()) {
                    return Err(format!("Field '{}' must contain only string values", self.name));
                }
                
                // Validate each value against options source if available
                if let Some(ref options_source) = self.validation.options_source {
                    match options_source {
                        OptionsSource::Fixed { options } => {
                            let valid_values: Vec<&str> = options.iter().map(|opt| opt.value.as_str()).collect();
                            for val in array {
                                let select_value = val.as_str().unwrap();
                                if !valid_values.contains(&select_value) {
                                    return Err(format!("Field '{}' values must be one of: {}", self.name, valid_values.join(", ")));
                                }
                            }
                        },
                        // For Enum and Query types, validation happens at DB level
                        _ => {}
                    }
                }
            },
            
            FieldType::Image | FieldType::File => {
                // Image/File type validation
                if !value.is_string() {
                    return Err(format!("Field '{}' must be a string path or ID", self.name));
                }
                // Additional validation could check that the file exists
            },
            
            FieldType::ManyToOne => {
                // ManyToOne relation validation
                if !value.is_number() && !value.is_string() {
                    return Err(format!("Field '{}' must be an entity ID or UUID", self.name));
                }
                
                // Additional validation could check that the related entity exists
                if self.validation.target_class.is_none() {
                    return Err(format!("Field '{}' is missing target class configuration", self.name));
                }
            },
            
            FieldType::ManyToMany => {
                // ManyToMany relation validation
                if !value.is_array() {
                    return Err(format!("Field '{}' must be an array of entity IDs or UUIDs", self.name));
                }
                
                // Check that all array elements are numbers or strings (IDs or UUIDs)
                let array = value.as_array().unwrap();
                if !array.iter().all(|v| v.is_number() || v.is_string()) {
                    return Err(format!("Field '{}' must contain only entity IDs or UUIDs", self.name));
                }
                
                if self.validation.target_class.is_none() {
                    return Err(format!("Field '{}' is missing target class configuration", self.name));
                }
            },
            
            FieldType::Object => {
                // Object type validation
                if !value.is_object() {
                    return Err(format!("Field '{}' must be a JSON object", self.name));
                }
                // Could add schema validation here
            },
            
            FieldType::Array => {
                // Array type validation
                if !value.is_array() {
                    return Err(format!("Field '{}' must be a JSON array", self.name));
                }
                // Could add item validation here
            },
        }
        
        Ok(())
    }
} 