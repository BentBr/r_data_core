use super::ImportAdapter;
use crate::workflow::data::adapters::AdapterContext;
use anyhow::Context;
use async_trait::async_trait;
use futures::{stream, Stream};
use serde_json::Value;

pub struct CsvImportAdapter;

impl CsvImportAdapter {
    /// Parse CSV inline content into an array of JSON objects according to format config.
    /// Supported config:
    /// - format.has_header: bool (default: true)
    /// - format.delimiter: string of length 1 (default: ",")
    /// - format.quote: string of length 1 (optional)
    /// - format.escape: string of length 1 (optional)
    pub fn parse_inline(
        inline: &str,
        format_cfg: &serde_json::Value,
    ) -> anyhow::Result<Vec<Value>> {
        let has_header = format_cfg
            .pointer("/has_header")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let delimiter = format_cfg
            .pointer("/delimiter")
            .and_then(|v| v.as_str())
            .and_then(|s| s.as_bytes().first().copied())
            .unwrap_or(b',');
        let quote = format_cfg
            .pointer("/quote")
            .and_then(|v| v.as_str())
            .and_then(|s| s.as_bytes().first().copied());
        let escape = format_cfg
            .pointer("/escape")
            .and_then(|v| v.as_str())
            .and_then(|s| s.as_bytes().first().copied());

        let mut builder = csv::ReaderBuilder::new();
        builder.has_headers(has_header);
        builder.delimiter(delimiter);
        if let Some(q) = quote {
            builder.quote(q);
        }
        if let Some(e) = escape {
            builder.escape(Some(e));
        }

        let mut rdr = builder.from_reader(inline.as_bytes());
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
        // Convert anyhow::Result<Value> vec to Value vec preserving early errors
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

#[async_trait]
impl ImportAdapter for CsvImportAdapter {
    async fn fetch_stream(
        &self,
        _ctx: &AdapterContext,
        cfg: &serde_json::Value,
    ) -> anyhow::Result<Box<dyn Stream<Item = anyhow::Result<Value>> + Unpin + Send>> {
        // Expect `source.inline` with CSV content for now.
        let inline = cfg
            .pointer("/source/inline")
            .and_then(|v| v.as_str())
            .context("missing source.inline for csv import adapter stub")?
            .to_string();

        let format_cfg = cfg
            .pointer("/format")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let parsed = Self::parse_inline(&inline, &format_cfg)?;

        Ok(Box::new(stream::iter(parsed.into_iter().map(Ok))))
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
