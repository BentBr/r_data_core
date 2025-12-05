use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CsvOptions {
    /// Respect header row (reading/writing)
    #[serde(default = "CsvOptions::default_header")]
    pub header: bool,
    /// Single-character delimiter, default ","
    #[serde(default = "CsvOptions::default_delimiter")]
    pub delimiter: String,
    /// Single-character escape sign, optional
    #[serde(default)]
    pub escape: Option<String>,
    /// Single-character quote, optional
    #[serde(default)]
    pub quote: Option<String>,
}

impl CsvOptions {
    #[allow(dead_code)] // Used by serde default
    pub const fn default_header() -> bool {
        true
    }
    #[allow(dead_code)] // Used by serde default
    pub fn default_delimiter() -> String {
        ",".to_string()
    }
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            header: true,
            delimiter: ",".to_string(),
            escape: None,
            quote: None,
        }
    }
}
