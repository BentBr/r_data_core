#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Job for fetching and staging workflow data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchAndStageJob {
    /// Workflow UUID
    pub workflow_id: Uuid,
    /// Optional trigger UUID
    pub trigger_id: Option<Uuid>,
}

/// Job for processing a raw item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRawItemJob {
    /// Raw item UUID
    pub raw_item_id: Uuid,
}
