pub mod uri;

use crate::data::adapters::auth::AuthProvider;
use anyhow::Result;
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
    async fn fetch(
        &self,
        ctx: &SourceContext,
    ) -> Result<Box<dyn Stream<Item = Result<Bytes>> + Unpin + Send>>;

    /// Validate source configuration
    fn validate(&self, config: &serde_json::Value) -> Result<()>;
}

/// Factory for creating source instances
pub trait SourceFactory: Send + Sync {
    fn source_type(&self) -> &'static str;
    fn create(&self, config: &serde_json::Value) -> Result<Box<dyn DataSource>>;
}
