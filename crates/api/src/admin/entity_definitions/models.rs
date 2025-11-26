#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

// Re-export PaginationQuery from the API crate's query module
pub use crate::query::PaginationQuery;

/// Path parameter for endpoints that accept a UUID in the URL
#[derive(Debug, Deserialize, ToSchema)]
pub struct PathUuid {
    /// UUID identifier from the URL path
    pub uuid: Uuid,
}

/// String field constraints
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, Clone)]
pub struct StringConstraints {
    /// Minimum string length
    pub min_length: Option<usize>,
    /// Maximum string length
    pub max_length: Option<usize>,
    /// Regex pattern for validation (e.g., "^[A-Z0-9]{2,20}$")
    pub pattern: Option<String>,
    /// Custom error message when validation fails
    pub error_message: Option<String>,
}

/// Numeric field constraints
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, Clone)]
pub struct NumericConstraints {
    /// Minimum allowed value
    pub min: Option<f64>,
    /// Maximum allowed value
    pub max: Option<f64>,
    /// Decimal precision for float values
    pub precision: Option<u8>,
    /// Whether only positive values are allowed
    pub positive_only: Option<bool>,
}

/// Date/time field constraints
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, Clone)]
pub struct DateTimeConstraints {
    /// Minimum allowed date
    pub min_date: Option<String>,
    /// Maximum allowed date
    pub max_date: Option<String>,
}

/// Select field constraints
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, Clone)]
pub struct SelectConstraints {
    /// Array of allowed values
    pub options: Option<Vec<String>>,
}

/// Relation field constraints
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, Clone)]
pub struct RelationConstraints {
    /// Name of the related entity type
    pub target_class: String,
}

/// Object/Array field constraints
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, Clone)]
pub struct SchemaConstraints {
    /// JSON schema for validating the object/array structure
    pub schema: Value,
}

/// Field constraints based on field type
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(tag = "type", content = "constraints")]
pub enum FieldConstraints {
    /// String field constraints
    #[serde(rename = "string")]
    String(StringConstraints),

    /// Integer field constraints
    #[serde(rename = "integer")]
    Integer(NumericConstraints),

    /// Float field constraints
    #[serde(rename = "float")]
    Float(NumericConstraints),

    /// Date/time field constraints
    #[serde(rename = "datetime")]
    DateTime(DateTimeConstraints),

    /// Date field constraints
    #[serde(rename = "date")]
    Date(DateTimeConstraints),

    /// Select field constraints
    #[serde(rename = "select")]
    Select(SelectConstraints),

    /// Multi-select field constraints
    #[serde(rename = "multiselect")]
    MultiSelect(SelectConstraints),

    /// Relation field constraints
    #[serde(rename = "relation")]
    Relation(RelationConstraints),

    /// Object/Array field constraints
    #[serde(rename = "schema")]
    Schema(SchemaConstraints),

    /// No constraints
    #[serde(rename = "none")]
    None,
}

/// Schema for options source in OpenAPI docs
/// Defines how to populate options for Select and MultiSelect fields
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum OptionsSourceSchema {
    /// Fixed list of options defined statically
    #[serde(rename = "fixed")]
    Fixed { options: Vec<SelectOptionSchema> },
    /// Options from a pre-defined enum type
    #[serde(rename = "enum")]
    Enum { enum_name: String },
    /// Options dynamically loaded from a database query
    #[serde(rename = "query")]
    Query {
        /// Target entity type to query
        entity_type: String,
        /// Field to use as option value
        value_field: String,
        /// Field to use as option display label
        label_field: String,
        /// Optional filter criteria for the query
        filter: Option<Value>,
    },
}

/// Schema for select options in OpenAPI docs
/// Used for defining individual options in fixed option lists
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SelectOptionSchema {
    /// Option value (stored in database)
    pub value: String,
    /// Option display label (shown in UI)
    pub label: String,
}

/// Schema for UI settings in OpenAPI docs
/// Controls how fields are rendered in forms and lists
#[derive(Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct UiSettingsSchema {
    /// Placeholder text shown in empty input fields
    pub placeholder: Option<String>,
    /// Help text shown below the field to provide additional context
    pub help_text: Option<String>,
    /// Whether to hide this field in list views
    pub hide_in_lists: Option<bool>,
    /// Layout width in grid units (1-12, where 12 is full width)
    pub width: Option<u8>,
    /// Field display order in forms (lower numbers appear first)
    pub order: Option<i32>,
    /// Group name for organizing fields into sections
    pub group: Option<String>,
    /// Custom CSS class to apply to the field container
    pub css_class: Option<String>,
    /// Configuration for WYSIWYG editor toolbar (for Wysiwyg fields)
    pub wysiwyg_toolbar: Option<String>,
    /// HTML input type attribute (e.g., "password", "email", "tel")
    pub input_type: Option<String>,
}

/// Field types available for entity definitions
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
    /// Type-specific field constraints
    #[serde(default)]
    pub constraints: Option<FieldConstraints>,
    /// UI settings for the field
    #[serde(default)]
    pub ui_settings: UiSettingsSchema,
}

/// Schema for entity definitions in OpenAPI docs
/// Used to define entity types with their fields and metadata
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(title = "EntityDefinitionSchema")]
pub struct EntityDefinitionSchema {
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
    /// Published &**state (whether visible to users)
    pub published: Option<bool>,
    /// Created at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Updated at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Response for listing entity definitions
#[derive(Debug, Serialize, ToSchema)]
#[schema(title = "EntityDefinitionListResponse")]
pub struct EntityDefinitionListResponse {
    /// List of entity definitions
    pub items: Vec<EntityDefinitionSchema>,
    /// Total number of items
    pub total: i64,
}

/// Model for apply-schema request
/// Used to generate and apply SQL schema for a specific entity definition or all definitions
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ApplySchemaRequest {
    /// Optional UUID of specific entity definition to apply schema for
    /// If not provided, schemas for all published entity definitions will be applied
    #[serde(default)]
    pub uuid: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
pub struct EntityDefinitionVersionMeta {
    pub version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct EntityDefinitionVersionPayload {
    pub version_number: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub data: serde_json::Value,
}

