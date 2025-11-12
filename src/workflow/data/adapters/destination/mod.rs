pub mod uri;

use crate::workflow::data::adapters::auth::AuthProvider;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// HTTP method for destinations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub fn requires_body(&self) -> bool {
        matches!(
            self,
            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Delete
        )
    }
}

/// Context for data destination operations
pub struct DestinationContext {
    pub auth: Option<Box<dyn AuthProvider>>,
    pub method: Option<HttpMethod>,
    pub config: serde_json::Value,
}

/// Trait for data destinations (URI, File, API, SFTP, etc.)
#[async_trait]
pub trait DataDestination: Send + Sync {
    /// Unique identifier for this destination type
    fn destination_type(&self) -> &'static str;

    /// Push data to the destination
    async fn push(&self, ctx: &DestinationContext, data: Bytes) -> Result<()>;

    /// Validate destination configuration
    fn validate(&self, config: &serde_json::Value) -> Result<()>;
}

/// Factory for creating destination instances
pub trait DestinationFactory: Send + Sync {
    fn destination_type(&self) -> &'static str;
    fn create(&self, config: &serde_json::Value) -> Result<Box<dyn DataDestination>>;
}
