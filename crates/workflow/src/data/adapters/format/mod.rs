pub mod csv;
pub mod json;

use anyhow::Result;
use bytes::Bytes;
use serde_json::Value;

/// Trait for format handlers (CSV, JSON, XML, Parquet, etc.)
pub trait FormatHandler: Send + Sync {
    /// Format identifier
    fn format_type(&self) -> &'static str;

    /// Parse data into JSON objects
    fn parse(&self, data: &[u8], options: &Value) -> Result<Vec<Value>>;

    /// Serialize JSON objects to format
    fn serialize(&self, data: &[Value], options: &Value) -> Result<Bytes>;

    /// Validate format configuration
    fn validate_options(&self, options: &Value) -> Result<()>;
}

/// Factory for creating format handlers
pub trait FormatFactory: Send + Sync {
    fn format_type(&self) -> &'static str;
    fn create(&self) -> Box<dyn FormatHandler>;
}
