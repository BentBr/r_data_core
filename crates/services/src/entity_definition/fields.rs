#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;

use super::EntityDefinitionService;
use super::ServiceEntityFieldInfo;

impl EntityDefinitionService {
    /// List all fields for a given entity type including system fields
    ///
    /// # Errors
    /// Returns an error if the entity type is not found or database query fails
    pub async fn list_fields_with_system_by_entity_type(
        &self,
        entity_type: &str,
    ) -> Result<Vec<ServiceEntityFieldInfo>> {
        let def = self
            .get_entity_definition_by_entity_type(entity_type)
            .await?;

        // Custom fields from definition
        let mut fields: Vec<ServiceEntityFieldInfo> = def
            .fields
            .iter()
            .map(|f| ServiceEntityFieldInfo {
                name: f.name.clone(),
                field_type: format!("{:?}", f.field_type),
                required: f.required,
                system: false,
            })
            .collect();

        // System fields common to all entities
        let mut system_fields = vec![
            ServiceEntityFieldInfo {
                name: "uuid".into(),
                field_type: "Uuid".into(),
                required: true,
                system: true,
            },
            ServiceEntityFieldInfo {
                name: "path".into(),
                field_type: "String".into(),
                required: false,
                system: true,
            },
            ServiceEntityFieldInfo {
                name: "created_at".into(),
                field_type: "DateTime".into(),
                required: true,
                system: true,
            },
            ServiceEntityFieldInfo {
                name: "updated_at".into(),
                field_type: "DateTime".into(),
                required: true,
                system: true,
            },
            ServiceEntityFieldInfo {
                name: "created_by".into(),
                field_type: "Uuid".into(),
                required: false,
                system: true,
            },
            ServiceEntityFieldInfo {
                name: "updated_by".into(),
                field_type: "Uuid".into(),
                required: false,
                system: true,
            },
            ServiceEntityFieldInfo {
                name: "published".into(),
                field_type: "Boolean".into(),
                required: true,
                system: true,
            },
            ServiceEntityFieldInfo {
                name: "version".into(),
                field_type: "Integer".into(),
                required: true,
                system: true,
            },
            ServiceEntityFieldInfo {
                name: "entity_key".into(),
                field_type: "String".into(),
                required: true,
                system: true,
            },
        ];
        fields.append(&mut system_fields);

        Ok(fields)
    }
}
