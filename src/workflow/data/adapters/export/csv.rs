use super::ExportAdapter;
use crate::workflow::data::adapters::AdapterContext;
use async_trait::async_trait;
use bytes::Bytes;
use futures::{stream, Stream};

pub struct CsvExportAdapter;

#[async_trait]
impl ExportAdapter for CsvExportAdapter {
    async fn stream(
        &self,
        _ctx: &AdapterContext,
        _cfg: &serde_json::Value,
    ) -> anyhow::Result<Box<dyn Stream<Item = anyhow::Result<Bytes>> + Unpin + Send>> {
        // Minimal stub stream with header + one example row
        let data: Vec<anyhow::Result<Bytes>> = vec![
            Ok(Bytes::from_static(b"id,name\n")),
            Ok(Bytes::from_static(b"1,Example\n")),
        ];
        Ok(Box::new(stream::iter(data)))
    }
}

pub struct CsvExportFactory;

impl super::ExportAdapterFactory for CsvExportFactory {
    fn name(&self) -> &'static str {
        "csv"
    }
    fn build(&self) -> Box<dyn super::ExportAdapter> {
        Box::new(CsvExportAdapter)
    }
}
