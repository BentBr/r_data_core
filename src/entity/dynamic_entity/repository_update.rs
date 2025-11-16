use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::entity::dynamic_entity::entity::DynamicEntity;
use crate::entity::dynamic_entity::repository::DynamicEntityRepository;
use crate::entity::dynamic_entity::utils;
use crate::error::{Error, Result};

pub async fn update_entity(repo: &DynamicEntityRepository, entity: &DynamicEntity) -> Result<()> {
    // Delegate to the existing impl in repository.rs (temporarily) to avoid duplication.
    // This function exists to start breaking the large repository into smaller modules.
    repo.update(entity).await
}
