use super::FormatHandler;
use bytes::Bytes;
use serde_json::Value;

/// CSV format handler
#[derive(Default)]
pub struct CsvFormatHandler;

impl CsvFormatHandler {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl FormatHandler for CsvFormatHandler {
    fn format_type(&self) -> &'static str {
        "csv"
    }

    /// # Errors
    /// Returns an error if CSV parsing fails.
    fn parse(&self, data: &[u8], options: &Value) -> r_data_core_core::error::Result<Vec<Value>> {
        let has_header = options
            .get("has_header")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let delimiter = options
            .get("delimiter")
            .and_then(|v| v.as_str())
            .and_then(|s| s.as_bytes().first().copied())
            .unwrap_or(b',');
        let quote = options
            .get("quote")
            .and_then(|v| v.as_str())
            .and_then(|s| s.as_bytes().first().copied());
        let escape = options
            .get("escape")
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

        let mut rdr = builder.from_reader(data);
        let headers = if has_header {
            Some(
                rdr.headers()
                    .map_err(|e| {
                        r_data_core_core::error::Error::Deserialization(format!(
                            "CSV header error: {e}"
                        ))
                    })?
                    .clone(),
            )
        } else {
            None
        };

        let mut rows: Vec<r_data_core_core::error::Result<Value>> = Vec::new();
        for result in rdr.records() {
            let rec = result.map_err(|e| {
                r_data_core_core::error::Error::Deserialization(format!("CSV record error: {e}"))
            })?;
            let mut obj = serde_json::Map::new();
            match &headers {
                Some(h) => {
                    for (i, field) in rec.iter().enumerate() {
                        let col_num = i + 1;
                        let key = h
                            .get(i)
                            .map_or_else(|| format!("col_{col_num}"), ToString::to_string);
                        obj.insert(key, serde_json::Value::String(field.to_string()));
                    }
                }
                None => {
                    for (i, field) in rec.iter().enumerate() {
                        let col_num = i + 1;
                        obj.insert(
                            format!("col_{col_num}"),
                            serde_json::Value::String(field.to_string()),
                        );
                    }
                }
            }
            rows.push(Ok(serde_json::Value::Object(obj)));
        }

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn serialize(&self, data: &[Value], options: &Value) -> r_data_core_core::error::Result<Bytes> {
        let has_header = options
            .get("has_header")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let delimiter = options
            .get("delimiter")
            .and_then(|v| v.as_str())
            .and_then(|s| s.as_bytes().first().copied())
            .unwrap_or(b',');
        let quote = options
            .get("quote")
            .and_then(|v| v.as_str())
            .and_then(|s| s.as_bytes().first().copied());
        let escape = options
            .get("escape")
            .and_then(|v| v.as_str())
            .and_then(|s| s.as_bytes().first().copied());

        let mut builder = csv::WriterBuilder::new();
        builder.has_headers(has_header);
        builder.delimiter(delimiter);
        if let Some(q) = quote {
            builder.quote(q);
        }
        if let Some(e) = escape {
            builder.escape(e);
        }
        let mut writer = builder.from_writer(Vec::new());

        // Write headers if present
        if has_header && !data.is_empty() {
            if let Some(obj) = data[0].as_object() {
                let headers: Vec<String> = obj.keys().cloned().collect();
                writer.write_record(&headers).map_err(|e| {
                    r_data_core_core::error::Error::Deserialization(format!(
                        "CSV write record error: {e}"
                    ))
                })?;
            }
        }

        // Write rows
        for value in data {
            if let Some(obj) = value.as_object() {
                let row: Vec<String> = obj
                    .values()
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        _ => v.to_string(),
                    })
                    .collect();
                writer.write_record(&row).map_err(|e| {
                    r_data_core_core::error::Error::Deserialization(format!(
                        "CSV write record error: {e}"
                    ))
                })?;
            }
        }

        writer.flush().map_err(|e| {
            r_data_core_core::error::Error::Deserialization(format!("CSV flush error: {e}"))
        })?;
        let bytes = writer.into_inner().map_err(|e| {
            r_data_core_core::error::Error::Deserialization(format!("CSV into_inner error: {e}"))
        })?;
        Ok(Bytes::from(bytes))
    }

    /// # Errors
    /// Returns an error if the configuration is invalid.
    fn validate_options(&self, options: &Value) -> r_data_core_core::error::Result<()> {
        if let Some(delimiter) = options.get("delimiter").and_then(|v| v.as_str()) {
            if delimiter.len() != 1 {
                return Err(r_data_core_core::error::Error::Validation(
                    "CSV delimiter must be a single character".to_string(),
                ));
            }
        }
        if let Some(quote) = options.get("quote").and_then(|v| v.as_str()) {
            if !quote.is_empty() && quote.len() != 1 {
                return Err(r_data_core_core::error::Error::Validation(
                    "CSV quote must be a single character when set".to_string(),
                ));
            }
        }
        if let Some(escape) = options.get("escape").and_then(|v| v.as_str()) {
            if !escape.is_empty() && escape.len() != 1 {
                return Err(r_data_core_core::error::Error::Validation(
                    "CSV escape must be a single character when set".to_string(),
                ));
            }
        }
        Ok(())
    }
}
