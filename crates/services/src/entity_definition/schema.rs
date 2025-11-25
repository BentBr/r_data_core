#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::error::Result;
use uuid::Uuid;

use super::EntityDefinitionService;

impl EntityDefinitionService {
    /// Cleanup unused entity tables
    ///
    /// # Errors
    /// Returns an error if cleanup operation fails
    pub async fn cleanup_unused_entity_tables(&self) -> Result<()> {
        self.repository.cleanup_unused_entity_view().await
    }

    /// Apply database schema for a specific entity definition or all if uuid is None
    ///
    /// # Errors
    /// Returns an error if schema application fails
    pub async fn apply_schema(
        &self,
        uuid: Option<&Uuid>,
    ) -> Result<(i32, Vec<(String, Uuid, String)>)> {
        if let Some(id) = uuid {
            // Apply schema for a specific entity definition
            let definition = self.get_entity_definition(id).await?;

            match self
                .repository
                .update_entity_view_for_entity_definition(&definition)
                .await
            {
                Ok(()) => Ok((1, Vec::new())),
                Err(e) => {
                    let failed = vec![(
                        definition.entity_type.clone(),
                        definition.uuid,
                        e.to_string(),
                    )];
                    Ok((0, failed))
                }
            }
        } else {
            // Apply schema for all entity definitions
            let definitions = self.list_entity_definitions(1000, 0).await?;
            let mut success_count = 0;
            let mut failed = Vec::new();

            for definition in definitions {
                match self
                    .repository
                    .update_entity_view_for_entity_definition(&definition)
                    .await
                {
                    Ok(()) => {
                        success_count += 1;
                    }
                    Err(e) => {
                        failed.push((
                            definition.entity_type.clone(),
                            definition.uuid,
                            e.to_string(),
                        ));
                    }
                }
            }

            Ok((success_count, failed))
        }
    }
}

