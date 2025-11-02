use super::super::adapters::AdapterContext;
use async_trait::async_trait;
use futures::Stream;

#[async_trait]
pub trait ImportAdapter: Send + Sync {
    async fn fetch_stream(
        &self,
        ctx: &AdapterContext,
        cfg: &serde_json::Value,
    ) -> anyhow::Result<Box<dyn Stream<Item = anyhow::Result<serde_json::Value>> + Unpin + Send>>;
}

pub trait ImportAdapterFactory: Send + Sync {
    fn name(&self) -> &'static str;
    fn build(&self) -> Box<dyn ImportAdapter>;
}

pub mod csv;
pub mod ndjson;
