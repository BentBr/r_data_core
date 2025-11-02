use super::ImportAdapter;
use crate::workflow::data::adapters::AdapterContext;
use anyhow::Context;
use async_trait::async_trait;
use futures::{stream, Stream};
use serde_json::Value;

pub struct NdjsonImportAdapter;

#[async_trait]
impl ImportAdapter for NdjsonImportAdapter {
    async fn fetch_stream(
        &self,
        _ctx: &AdapterContext,
        cfg: &serde_json::Value,
    ) -> anyhow::Result<Box<dyn Stream<Item = anyhow::Result<Value>> + Unpin + Send>> {
        // Minimal stub: expect `source.inline` with NDJSON content for now.
        let inline = cfg
            .pointer("/source/inline")
            .and_then(|v| v.as_str())
            .context("missing source.inline for ndjson import adapter stub")?
            .to_string();

        let rows = inline
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str::<Value>(l).map_err(anyhow::Error::from))
            .map(|r| r.map_err(anyhow::Error::from))
            .collect::<Vec<_>>();

        Ok(Box::new(stream::iter(rows)))
    }
}

pub struct NdjsonImportFactory;

impl super::ImportAdapterFactory for NdjsonImportFactory {
    fn name(&self) -> &'static str {
        "ndjson"
    }
    fn build(&self) -> Box<dyn super::ImportAdapter> {
        Box::new(NdjsonImportAdapter)
    }
}
