use crate::workflow::data::adapters::AdapterContext;
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;

#[async_trait]
pub trait ExportAdapter: Send + Sync {
    async fn stream(
        &self,
        ctx: &AdapterContext,
        cfg: &serde_json::Value,
    ) -> anyhow::Result<Box<dyn Stream<Item = anyhow::Result<Bytes>> + Unpin + Send>>;
}

pub trait ExportAdapterFactory: Send + Sync {
    fn name(&self) -> &'static str;
    fn build(&self) -> Box<dyn ExportAdapter>;
}

pub mod csv;
