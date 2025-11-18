use sqlx::PgPool;
use uuid::Uuid;

use crate::entity::version_repository::VersionRepository;
use crate::error::Result;

/// Service for managing entity versions with business logic
pub struct VersionService {
    version_repo: VersionRepository,
}

impl VersionService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            version_repo: VersionRepository::new(pool),
        }
    }

    /// List all versions for an entity, including the current version if not in versions table.
    /// Creator names are resolved via SQL JOINs in the repository.
    pub async fn list_entity_versions_with_metadata(
        &self,
        entity_uuid: Uuid,
    ) -> Result<Vec<VersionMetaWithName>> {
        // Get historical versions from versions table (with creator names from JOINs)
        let rows = self.version_repo.list_entity_versions(entity_uuid).await?;

        // Get current entity metadata from registry (with creator name from JOIN)
        let current_metadata = self
            .version_repo
            .get_current_entity_metadata(entity_uuid)
            .await?;

        let mut out: Vec<VersionMetaWithName> = Vec::new();

        // Add current version if it exists and is not in the versions table
        if let Some((version, updated_at, updated_by, updated_by_name)) = current_metadata {
            let is_in_versions = rows.iter().any(|r| r.version_number == version);
            if !is_in_versions {
                out.push(VersionMetaWithName {
                    version_number: version,
                    created_at: updated_at,
                    created_by: updated_by,
                    created_by_name: updated_by_name,
                });
            }
        }

        // Add all historical versions (creator names already resolved via JOINs)
        for r in rows {
            out.push(VersionMetaWithName {
                version_number: r.version_number,
                created_at: r.created_at,
                created_by: r.created_by,
                created_by_name: r.created_by_name,
            });
        }

        // Sort by version number descending (newest first)
        out.sort_by(|a, b| b.version_number.cmp(&a.version_number));

        Ok(out)
    }
}

/// Version metadata with resolved creator name
#[derive(Debug, Clone)]
pub struct VersionMetaWithName {
    pub version_number: i32,
    pub created_at: time::OffsetDateTime,
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
}

