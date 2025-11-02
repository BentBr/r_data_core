use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchAndStageJob {
    pub workflow_id: Uuid,
    pub trigger_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRawItemJob {
    pub raw_item_id: Uuid,
}
