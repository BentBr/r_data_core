use serde::{Deserialize, Serialize};
use sqlx::{
    decode::Decode,
    postgres::{PgTypeInfo, PgValueRef},
    Type,
};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::{Uuid};

use super::class::ClassDefinition;
use super::DynamicFields;
use super::VersionedData;
use crate::error::{Error, Result};

/// The base entity from which all RDataEntities derive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractRDataEntity {
    /// Public UUID for API consumption
    pub uuid: Uuid,

    /// Path for object tree organization
    pub path: String,

    /// When the entity was created
    pub created_at: OffsetDateTime,

    /// When the entity was last modified
    pub updated_at: OffsetDateTime,

    /// Who created the entity
    pub created_by: Uuid,

    /// Who last modified the entity
    pub updated_by: Option<Uuid>,

    /// Entity published status
    pub published: bool,

    /// Current version number
    pub version: i32,

    /// Custom fields storage
    #[serde(default)]
    pub custom_fields: HashMap<String, serde_json::Value>,
}

impl Type<sqlx::Postgres> for AbstractRDataEntity {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("JSONB")
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for AbstractRDataEntity {
    fn decode(value: PgValueRef<'r>) -> std::result::Result<Self, sqlx::error::BoxDynError> {
        // For JSON representation, decode as a JSON value first
        let json_value = <serde_json::Value as Decode<'r, sqlx::Postgres>>::decode(value)?;

        // Then deserialize from the JSON value
        let entity = serde_json::from_value(json_value)?;
        Ok(entity)
    }
}

impl AbstractRDataEntity {
    /// Create a new entity with default values
    pub fn new(path: String) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            uuid: Uuid::now_v7(),
            path,
            created_at: now,
            updated_at: now,
            created_by: Uuid::nil(),
            updated_by: None,
            published: false,
            version: 1,
            custom_fields: HashMap::new(),
        }
    }

    /// Get the full path including entity name
    pub fn full_path(&self) -> String {
        if self.path.ends_with('/') {
            format!("{}{}", self.path, self.uuid)
        } else {
            format!("{}/{}", self.path, self.uuid)
        }
    }

    /// Increment version when entity is updated
    pub fn increment_version(&mut self) {
        self.version += 1;
        self.updated_at = OffsetDateTime::now_utc();
    }

    /// Create a versioned snapshot of the current entity state
    pub fn create_version_snapshot(&self) -> VersionedData {
        VersionedData {
            entity_uuid: self.uuid,
            version_number: self.version,
            data: serde_json::to_value(self).unwrap_or(serde_json::Value::Null),
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

impl DynamicFields for AbstractRDataEntity {
    fn get_field(&self, name: &str) -> Option<serde_json::Value> {
        self.custom_fields.get(name).cloned()
    }

    fn set_field(&mut self, name: &str, value: serde_json::Value) -> Result<()> {
        self.custom_fields.insert(name.to_string(), value);
        Ok(())
    }

    fn get_all_fields(&self) -> HashMap<String, serde_json::Value> {
        self.custom_fields.clone()
    }

    fn validate(&self, class_def: &ClassDefinition) -> Result<()> {
        // Basic validation - ensure all required fields are present
        if let Some(properties) = class_def.schema.properties.get("properties") {
            if let Some(props) = properties.as_object() {
                for (field_name, field_def) in props {
                    if let Some(required) = field_def.get("required") {
                        if required.as_bool() == Some(true)
                            && !self.custom_fields.contains_key(field_name)
                        {
                            return Err(Error::Validation(format!(
                                "Required field '{}' is missing",
                                field_name
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
