use sqlx::PgPool;
use uuid::Uuid;

use crate::error::Result;
use crate::entity::version_repository::VersionRepository;

pub struct VersioningService {
    pool: PgPool,
}

impl VersioningService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Save a pre-update snapshot of an entity
    pub async fn save_pre_update_snapshot(
        &self,
        entity_uuid: Uuid,
        entity_type: &str,
        current_version: i32,
        data: &serde_json::Value,
        user: Option<Uuid>,
    ) -> Result<()> {
        let repo = VersionRepository::new(self.pool.clone());
        repo.insert_snapshot(entity_uuid, entity_type, current_version, data.clone(), user)
            .await
    }
}


