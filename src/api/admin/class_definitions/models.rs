use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Pagination query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Path parameter for UUID
#[derive(Debug, Deserialize, ToSchema)]
pub struct PathUuid {
    pub uuid: Uuid,
}

/// Schema for field validation in OpenAPI docs
#[derive(Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct FieldValidationSchema {
    /// Minimum string length
    pub min_length: Option<usize>,
    /// Maximum string length
    pub max_length: Option<usize>,
    /// Regex pattern for string validation
    pub pattern: Option<String>,
    /// Minimum numeric value
    pub min_value: Option<Value>,
    /// Maximum numeric value
    pub max_value: Option<Value>,
    /// Allow only positive values for numeric fields
    pub positive_only: Option<bool>,
    /// Minimum date (ISO string or "now")
    pub min_date: Option<String>,
    /// Maximum date (ISO string or "now")
    pub max_date: Option<String>,
    /// Target class for relation fields
    pub target_class: Option<String>,
    /// Options source for select fields
    pub options_source: Option<OptionsSourceSchema>,
}

/// Schema for options source in OpenAPI docs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum OptionsSourceSchema {
    /// Fixed list of options
    #[serde(rename = "fixed")]
    Fixed { options: Vec<SelectOptionSchema> },
    /// Options from an enum
    #[serde(rename = "enum")]
    Enum { enum_name: String },
    /// Options from a database query
    #[serde(rename = "query")]
    Query {
        entity_type: String,
        value_field: String,
        label_field: String,
        filter: Option<Value>,
    },
}

/// Schema for select options in OpenAPI docs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SelectOptionSchema {
    /// Option value
    pub value: String,
    /// Option display label
    pub label: String,
}

/// Schema for UI settings in OpenAPI docs
#[derive(Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct UiSettingsSchema {
    /// Placeholder text
    pub placeholder: Option<String>,
    /// Help text
    pub help_text: Option<String>,
    /// Hide in list views
    pub hide_in_lists: Option<bool>,
    /// Layout width (1-12)
    pub width: Option<u8>,
    /// Display order
    pub order: Option<i32>,
    /// Group name
    pub group: Option<String>,
    /// CSS class
    pub css_class: Option<String>,
    /// WYSIWYG toolbar config
    pub wysiwyg_toolbar: Option<String>,
    /// Input type (e.g., "password")
    pub input_type: Option<String>,
}

/// Field types available for class definitions
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub enum FieldTypeSchema {
    String,
    Text,
    Wysiwyg,
    Integer,
    Float,
    Boolean,
    DateTime,
    Date,
    Object,
    Array,
    Uuid,
    ManyToOne,
    ManyToMany,
    Select,
    MultiSelect,
    Image,
    File,
}

/// Schema for field definitions in OpenAPI docs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FieldDefinitionSchema {
    /// Field name (must be unique within class)
    pub name: String,
    /// User-friendly display name
    pub display_name: String,
    /// Field data type
    pub field_type: FieldTypeSchema,
    /// Field description
    pub description: Option<String>,
    /// Whether the field is required
    pub required: bool,
    /// Whether the field is indexed for faster searches
    pub indexed: bool,
    /// Whether the field can be used in API filtering
    pub filterable: bool,
    /// Default value for the field
    pub default_value: Option<Value>,
    /// Field validation/constraints
    #[serde(default)]
    pub validation: FieldValidationSchema,
    /// UI settings for the field
    #[serde(default)]
    pub ui_settings: UiSettingsSchema,
    /// Extra constraints
    pub constraints: Option<HashMap<String, Value>>,
}

/// Schema for class definitions in OpenAPI docs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(title = "ClassDefinitionSchema")]
pub struct ClassDefinitionSchema {
    /// Unique identifier
    pub uuid: Option<Uuid>,
    /// Entity type name (must be unique)
    pub entity_type: String,
    /// Display name for this entity type
    pub display_name: String,
    /// Description of this entity type
    pub description: Option<String>,
    /// Group name for organizing entity types
    pub group_name: Option<String>,
    /// Whether this entity type can have children
    pub allow_children: bool,
    /// Icon for this entity type
    pub icon: Option<String>,
    /// Field definitions for this entity type
    pub fields: Vec<FieldDefinitionSchema>,
    /// Published state (whether visible to users)
    pub published: Option<bool>,
}

/// Response for listing class definitions
#[derive(Debug, Serialize, ToSchema)]
#[schema(title = "ClassDefinitionListResponse")]
pub struct ClassDefinitionListResponse {
    /// List of class definitions
    pub items: Vec<ClassDefinitionSchema>,
    /// Total number of items
    pub total: i64,
}

/// Model for apply-schema request
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ApplySchemaRequest {
    #[serde(default)]
    pub uuid: Option<Uuid>,
}
