#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::collections::HashMap;

use r_data_core_core::entity_definition::definition::EntityDefinition;
use r_data_core_core::error::Result;
use r_data_core_core::field::types::FieldType;
use r_data_core_core::DynamicEntity;
use r_data_core_persistence::FilterEntitiesParams;
use serde_json::Value as JsonValue;

use super::DynamicEntityService;

impl DynamicEntityService {
    /// Filter entities based on field values
    ///
    /// # Errors
    /// Returns an error if entity type is not found, not published, or database query fails
    #[allow(clippy::too_many_arguments)] // Public API - parameters are clear and well-named
    pub async fn filter_entities(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        filters: Option<HashMap<String, JsonValue>>,
        search: Option<(String, Vec<String>)>,
        sort: Option<(String, String)>,
        fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        self.filter_entities_with_operators(
            entity_type,
            limit,
            offset,
            filters,
            None, // Default to "=" for all filters
            search,
            sort,
            fields,
        )
        .await
    }

    /// Filter entities based on field values with operators
    ///
    /// # Errors
    /// Returns an error if entity type is not found, not published, or database query fails
    #[allow(clippy::too_many_arguments)] // Public API - parameters are clear and well-named
    pub async fn filter_entities_with_operators(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        filters: Option<HashMap<String, JsonValue>>,
        filter_operators: Option<HashMap<String, String>>,
        search: Option<(String, Vec<String>)>,
        sort: Option<(String, String)>,
        fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        // Verify the entity type exists and is published
        self.check_entity_type_exists_and_published(entity_type)
            .await?;

        let params = FilterEntitiesParams::new(limit, offset)
            .with_filters(filters)
            .with_filter_operators(filter_operators)
            .with_search(search)
            .with_sort(sort)
            .with_fields(fields);
        self.repository.filter_entities(entity_type, &params).await
    }

    /// List entities with advanced filtering options
    ///
    /// # Errors
    /// Returns an error if entity type is not found, not published, or database query fails
    #[allow(clippy::too_many_arguments)] // Public API - parameters are clear and well-named
    pub async fn list_entities_with_filters(
        &self,
        entity_type: &str,
        limit: i64,
        offset: i64,
        fields: Option<Vec<String>>,
        sort_by: Option<String>,
        sort_direction: Option<String>,
        filter: Option<serde_json::Value>,
        search_query: Option<String>,
    ) -> Result<(Vec<DynamicEntity>, i64)> {
        // Verify the entity type exists and is published
        let entity_def = self.get_entity_definition_for_query(entity_type).await?;

        // Count entities first for pagination
        let total = self.repository.count_entities(entity_type).await?;

        // Build filter conditions from the structured filter
        let mut filter_conditions = HashMap::new();

        if let Some(ref filter_value) = filter {
            if let Some(obj) = filter_value.as_object() {
                for (key, value) in obj {
                    filter_conditions.insert(key.clone(), value.clone());
                }
            }
        }

        // Normalize folder listing semantics for path-based browsing
        // Support: { path: "/" } or { path: "/myFolder" } to mean: list items directly under that folder
        // Transform into SQL-friendly conditions using special keys handled in repository:
        // - path_equals for exact folder entity
        // - path_prefix for recursive children under folder
        if let Some(value_obj) = &filter {
            if let Some(obj) = value_obj.as_object() {
                if let Some(path_val) = obj.get("path").and_then(|v| v.as_str()) {
                    let normalized = if path_val.is_empty() { "/" } else { path_val };
                    // Remove original generic path if present
                    filter_conditions.remove("path");
                    // Add explicit path filters; FE can decide which to use, for now include prefix
                    filter_conditions
                        .insert("path_prefix".to_string(), serde_json::json!(normalized));
                }
            }
        }

        // Add search query if provided
        let search_fields = if let Some(query) = search_query {
            // Get text/string fields from entity definition for searching
            let searchable_fields: Vec<String> = entity_def
                .fields
                .iter()
                .filter(|field| {
                    matches!(
                        field.field_type,
                        FieldType::String | FieldType::Text | FieldType::Wysiwyg
                    )
                })
                .map(|field| field.name.clone())
                .collect();

            // Return the query and fields to search in
            if searchable_fields.is_empty() {
                None
            } else {
                Some((query, searchable_fields))
            }
        } else {
            None
        };

        // Build sort information
        let sort_info = sort_by.map_or_else(
            || Some(("created_at".to_string(), "DESC".to_string())),
            |field| {
                let direction = sort_direction.unwrap_or_else(|| "ASC".to_string());
                Some((field, direction))
            },
        );

        // Fetch the entities
        let params = FilterEntitiesParams::new(limit, offset)
            .with_filters(Some(filter_conditions))
            .with_filter_operators(None) // Default to "=" for all filters
            .with_search(search_fields)
            .with_sort(sort_info)
            .with_fields(fields);
        let entities = self
            .repository
            .filter_entities(entity_type, &params)
            .await?;

        Ok((entities, total))
    }

    /// Helper method to get entity definition for query operations
    ///
    /// # Arguments
    /// * `entity_type` - Entity type string
    ///
    /// # Errors
    /// Returns an error if entity type is not found or not published
    async fn get_entity_definition_for_query(&self, entity_type: &str) -> Result<EntityDefinition> {
        // Look up the entity definition
        let entity_def = match self
            .entity_definition_service
            .get_entity_definition_by_entity_type(entity_type)
            .await
        {
            Ok(entity_def) => entity_def,
            Err(r_data_core_core::error::Error::NotFound(_)) => {
                return Err(r_data_core_core::error::Error::NotFound(format!(
                    "Entity type '{entity_type}' not found"
                )));
            }
            Err(e) => return Err(e),
        };

        // Ensure the class is published
        if !entity_def.published {
            return Err(r_data_core_core::error::Error::ValidationFailed(format!(
                "Entity type '{entity_type}' is not published"
            )));
        }

        Ok(entity_def)
    }

    #[allow(dead_code)]
    async fn get_entities_with_filters(
        &self,
        entity_type: &str,
        filters: Option<HashMap<String, JsonValue>>,
        limit: i64,
        offset: i64,
        exclusive_fields: Option<Vec<String>>,
    ) -> Result<Vec<DynamicEntity>> {
        // If no filters, use the standard method
        if filters.is_none() {
            return self
                .repository
                .get_all_by_type(entity_type, limit, offset, exclusive_fields)
                .await;
        }

        // Validate entity type
        let _ = self.get_entity_definition_for_query(entity_type).await?;

        // Use the new filter_entities method with the structured parameters
        let params = FilterEntitiesParams::new(limit, offset)
            .with_filters(filters)
            .with_filter_operators(None) // Default to "=" for all filters
            .with_fields(exclusive_fields);
        self.repository.filter_entities(entity_type, &params).await
    }
}
