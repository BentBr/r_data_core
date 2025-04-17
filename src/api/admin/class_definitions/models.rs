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
/// Used for string pattern validation via regex. For other field type validations,
/// use the constraints field instead.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct FieldValidationSchema {
    /// Regex pattern for validating string fields (e.g., "^[A-Za-z0-9_]+$" for alphanumeric validation)
    pub pattern: Option<String>,
    /// Custom error message to display when validation fails
    pub error_message: Option<String>,
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
pub enum FieldTypeSchema {
    /// Short text field (varchar in database)
    String,
    /// Long text field (text in database)
    Text,
    /// Rich text editor field (text in database, HTML content)
    Wysiwyg,
    /// Whole number field (integer in database)
    Integer,
    /// Decimal number field (float in database)
    Float,
    /// True/false field (boolean in database)
    Boolean,
    /// Date and time field (timestamp in database)
    DateTime,
    /// Date only field (date in database)
    Date,
    /// JSON object field (jsonb in database)
    Object,
    /// JSON array field (jsonb in database)
    Array,
    /// UUID field (uuid in database)
    Uuid,
    /// Reference to a single related entity (foreign key)
    ManyToOne,
    /// Reference to multiple related entities (junction table)
    ManyToMany,
    /// Single option from a predefined list (enum or lookup)
    Select,
    /// Multiple options from a predefined list (array in database)
    MultiSelect,
    /// Image upload field (stores file reference)
    Image,
    /// File upload field (stores file reference)
    File,
}

/// Schema for field definitions in OpenAPI docs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FieldDefinitionSchema {
    /// Field name (must be unique within class and contain only alphanumeric characters, underscores, no spaces)
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
    /// Extra constraints specific to field type:
    /// - String/Text: min_length, max_length
    /// - Integer/Float: min, max, positive_only
    /// - DateTime/Date: min_date, max_date
    /// - ManyToOne/ManyToMany: target_class
    /// - Select/MultiSelect: options_source
    /// - Object/Array: schema
    pub constraints: Option<HashMap<String, Value>>,
}

/// Schema for class definitions in OpenAPI docs
/// Used to define entity types with their fields and metadata
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(title = "ClassDefinitionSchema")]
pub struct ClassDefinitionSchema {
    /// Unique identifier (automatically generated if not provided)
    pub uuid: Option<Uuid>,
    /// Entity type name (must be unique, alphanumeric with underscores, no spaces)
    pub entity_type: String,
    /// User-friendly display name for this entity type
    pub display_name: String,
    /// Description of this entity type
    pub description: Option<String>,
    /// Group name for organizing entity types
    pub group_name: Option<String>,
    /// Whether this entity type can have children
    pub allow_children: bool,
    /// Icon identifier for this entity type
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
/// Used to generate and apply SQL schema for a specific class definition or all definitions
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ApplySchemaRequest {
    /// Optional UUID of specific class definition to apply schema for
    /// If not provided, schemas for all published class definitions will be applied
    #[serde(default)]
    pub uuid: Option<Uuid>,
}
