pub mod uri;

use crate::data::adapters::auth::AuthProvider;
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;

/// Context for data source operations
pub struct SourceContext {
    pub auth: Option<Box<dyn AuthProvider>>,
    pub config: serde_json::Value,
}

/// Trait for data sources (URI, File, API, SFTP, etc.)
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Unique identifier for this source type
    fn source_type(&self) -> &'static str;

    /// Fetch data from the source
    ///
    /// # Errors
    /// Returns an error if the fetch operation fails.
    async fn fetch(
        &self,
        ctx: &SourceContext,
    ) -> r_data_core_core::error::Result<
        Box<dyn Stream<Item = r_data_core_core::error::Result<Bytes>> + Unpin + Send>,
    >;

    /// Validate source configuration
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid.
    fn validate(&self, config: &serde_json::Value) -> r_data_core_core::error::Result<()>;
}

/// Factory for creating source instances
pub trait SourceFactory: Send + Sync {
    fn source_type(&self) -> &'static str;
    /// # Errors
    /// Returns an error if the source cannot be created from the config.
    fn create(
        &self,
        config: &serde_json::Value,
    ) -> r_data_core_core::error::Result<Box<dyn DataSource>>;
}
