use super::ImportAdapter;
use crate::workflow::data::adapters::AdapterContext;
use anyhow::Context;
use async_trait::async_trait;
use futures::{stream, Stream};
use serde_json::Value;

pub struct CsvImportAdapter;

#[async_trait]
impl ImportAdapter for CsvImportAdapter {
    async fn fetch_stream(
        &self,
        _ctx: &AdapterContext,
        cfg: &serde_json::Value,
    ) -> anyhow::Result<Box<dyn Stream<Item = anyhow::Result<Value>> + Unpin + Send>> {
        // Minimal stub: expect `source.inline` with CSV content for now.
        let inline = cfg
            .pointer("/source/inline")
            .and_then(|v| v.as_str())
            .context("missing source.inline for csv import adapter stub")?
            .to_string();

        let has_header = cfg
            .pointer("/format/has_header")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let delimiter = cfg
            .pointer("/format/delimiter")
            .and_then(|v| v.as_str())
            .and_then(|s| s.as_bytes().first().copied())
            .unwrap_or(b',');

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(has_header)
            .delimiter(delimiter)
            .from_reader(inline.as_bytes());

        let headers = if has_header {
            Some(rdr.headers()?.clone())
        } else {
            None
        };

        let mut rows: Vec<anyhow::Result<Value>> = Vec::new();
        for result in rdr.records() {
            let rec = result?;
            let mut obj = serde_json::Map::new();
            match &headers {
                Some(h) => {
                    for (i, field) in rec.iter().enumerate() {
                        let key = h
                            .get(i)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("col_{}", i + 1));
                        obj.insert(key, Value::String(field.to_string()));
                    }
                }
                None => {
                    for (i, field) in rec.iter().enumerate() {
                        obj.insert(format!("col_{}", i + 1), Value::String(field.to_string()));
                    }
                }
            }
            rows.push(Ok(Value::Object(obj)));
        }

        Ok(Box::new(stream::iter(rows)))
    }
}

pub struct CsvImportFactory;

impl super::ImportAdapterFactory for CsvImportFactory {
    fn name(&self) -> &'static str {
        "csv"
    }
    fn build(&self) -> Box<dyn super::ImportAdapter> {
        Box::new(CsvImportAdapter)
    }
}
