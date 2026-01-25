pub mod uri;

use crate::data::adapters::auth::AuthProvider;
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
    #[must_use]
    pub const fn requires_body(&self) -> bool {
        matches!(self, Self::Post | Self::Put | Self::Patch | Self::Delete)
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
    ///
    /// # Errors
    /// Returns an error if the push operation fails.
    async fn push(
        &self,
        ctx: &DestinationContext,
        data: Bytes,
    ) -> r_data_core_core::error::Result<()>;

    /// Validate destination configuration
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid.
    fn validate(&self, config: &serde_json::Value) -> r_data_core_core::error::Result<()>;
}

/// Factory for creating destination instances
pub trait DestinationFactory: Send + Sync {
    fn destination_type(&self) -> &'static str;
    /// # Errors
    /// Returns an error if the destination cannot be created from the config.
    fn create(
        &self,
        config: &serde_json::Value,
    ) -> r_data_core_core::error::Result<Box<dyn DataDestination>>;
}
