pub mod csv;
pub mod json;

use bytes::Bytes;
use serde_json::Value;

/// Trait for format handlers (CSV, JSON, XML, Parquet, etc.)
pub trait FormatHandler: Send + Sync {
    /// Format identifier
    fn format_type(&self) -> &'static str;

    /// Parse data into JSON objects
    ///
    /// # Errors
    /// Returns an error if parsing fails.
    fn parse(&self, data: &[u8], options: &Value) -> r_data_core_core::error::Result<Vec<Value>>;

    /// Serialize JSON objects to format
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    fn serialize(&self, data: &[Value], options: &Value) -> r_data_core_core::error::Result<Bytes>;

    /// Validate format configuration
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid.
    fn validate_options(&self, options: &Value) -> r_data_core_core::error::Result<()>;
}

/// Factory for creating format handlers
pub trait FormatFactory: Send + Sync {
    fn format_type(&self) -> &'static str;
    fn create(&self) -> Box<dyn FormatHandler>;
}
