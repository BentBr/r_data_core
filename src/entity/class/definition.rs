use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, PgPool, Row};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::{Timestamp, Uuid};

use super::schema::Schema;
use crate::db::create_or_update_enum;
use crate::entity::field::{get_sql_type_for_field, FieldDefinition, FieldType, OptionsSource};
use crate::entity::AbstractRDataEntity;
use crate::error::{Error, Result};

/// Class definition for a custom RDataEntity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClassDefinition {
    /// Primary key
    pub uuid: Uuid,

    /// Entity type (unique identifier)
    pub entity_type: String,

    /// JSON schema for the class
    pub schema: Schema,

    /// When the class was created
    pub created_at: DateTime<Utc>,

    /// When the class was last updated
    pub updated_at: DateTime<Utc>,
}

// Implement FromRow for ClassDefinition
impl<'r> FromRow<'r, PgRow> for ClassDefinition {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        // Extract the base entity data
        let id = row.try_get::<i64, _>("id")?;
        let uuid = row.try_get::<String, _>("uuid")?;
        let path = row.try_get::<String, _>("path")?;
        let created_at = row
            .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")?;
        let updated_at = row
            .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("updated_at")?;
        let created_by = row.try_get::<Option<Uuid>, _>("created_by").unwrap_or(None);
        let updated_by = row.try_get::<Option<Uuid>, _>("updated_by").unwrap_or(None);
        let published = row.try_get::<bool, _>("published").unwrap_or(false);
        let version = row.try_get::<i32, _>("version").unwrap_or(1);

        // Extract class definition specific fields
        let class_name = row.try_get::<String, _>("class_name")?;
        let display_name = row.try_get::<String, _>("display_name")?;
        let description = row
            .try_get::<Option<String>, _>("description")
            .unwrap_or(None);
        let group = row
            .try_get::<Option<String>, _>("group_name")
            .unwrap_or(None);
        let allow_children = row.try_get::<bool, _>("allow_children").unwrap_or(false);
        let icon = row.try_get::<Option<String>, _>("icon").unwrap_or(None);

        // Fields are stored as JSON
        let fields_json = row
            .try_get::<serde_json::Value, _>("fields")
            .unwrap_or(serde_json::json!([]));

        let fields: Vec<FieldDefinition> =
            serde_json::from_value(fields_json).map_err(|e| sqlx::Error::ColumnDecode {
                index: "fields".to_string(),
                source: Box::new(e),
            })?;

        // Get custom fields as JSON Value first
        let custom_fields_json = row
            .try_get::<serde_json::Value, _>("custom_fields")
            .unwrap_or(serde_json::json!({}));

        // Convert to HashMap
        let custom_fields =
            serde_json::from_value(custom_fields_json).unwrap_or_else(|_| HashMap::new());

        // Build and return the entity
        let base = AbstractRDataEntity {
            uuid: Uuid::parse_str(&uuid).map_err(|e| sqlx::Error::ColumnDecode {
                index: "uuid".to_string(),
                source: Box::new(e),
            })?,
            path,
            created_at,
            updated_at,
            created_by,
            updated_by,
            published,
            version,
            custom_fields,
        };

        let now = Utc::now();
        let ts = Timestamp::from_unix(now.timestamp(), now.timestamp_subsec_nanos() as u64, 0);

        Ok(ClassDefinition {
            uuid: Uuid::new_v7(ts),
            entity_type: class_name,
            schema: Schema::new(fields, base),
            created_at,
            updated_at,
        })
    }
}

impl ClassDefinition {
    /// Create a new class definition
    pub fn new(entity_type: String, schema: Schema) -> Self {
        let now = Utc::now();
        let ts = Timestamp::from_unix(now.timestamp(), now.timestamp_subsec_nanos() as u64, 0);

        Self {
            uuid: Uuid::new_v7(ts),
            entity_type,
            schema,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the SQL table name for this entity type
    pub fn get_table_name(&self) -> String {
        format!("{}_entities", self.entity_type.to_lowercase())
    }

    /// Apply the class definition to the database
    pub async fn apply_to_database(&self, db: &PgPool) -> Result<()> {
        let sql = self.schema.generate_sql_schema();
        sqlx::query(&sql).execute(db).await?;
        Ok(())
    }

    /// Add a new enum with values
    pub async fn add_enum_with_values(
        &self,
        db: &PgPool,
        enum_name: &str,
        values: &[String],
    ) -> Result<()> {
        // Create or update the enum type
        crate::db::create_or_update_enum(db, enum_name, values).await?;
        Ok(())
    }

    /// Add an enum field to the database with its values
    pub async fn add_enum_field(
        &self,
        db: &PgPool,
        _field_name: &str,
        enum_name: &str,
        values: &[String],
    ) -> Result<()> {
        // Add enum type first
        create_or_update_enum(db, enum_name, values).await
    }

    pub fn generate_sql_schema(&self) -> String {
        self.schema.generate_sql_schema()
    }
}
