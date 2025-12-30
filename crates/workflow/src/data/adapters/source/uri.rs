use super::{DataSource, SourceContext};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{stream, Stream};

/// URI-based data source (HTTP/HTTPS)
#[derive(Default)]
pub struct UriSource;

impl UriSource {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DataSource for UriSource {
    fn source_type(&self) -> &'static str {
        "uri"
    }

    async fn fetch(
        &self,
        ctx: &SourceContext,
    ) -> r_data_core_core::error::Result<Box<dyn Stream<Item = r_data_core_core::error::Result<Bytes>> + Unpin + Send>> {
        let uri = ctx
            .config
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| r_data_core_core::error::Error::Config("URI source requires 'uri' in config".to_string()))?;

        let client = reqwest::Client::new();
        let mut request = client.get(uri);

        // Apply authentication if provided
        if let Some(auth) = &ctx.auth {
            request = auth.apply_to_request(request)
                .map_err(|e| r_data_core_core::error::Error::Api(e.to_string()))?;
        }

        let response = request.send().await
            .map_err(|e| r_data_core_core::error::Error::Api(format!("Failed to send request: {e}")))?;
        let response = response.error_for_status()
            .map_err(|e| r_data_core_core::error::Error::Api(format!("HTTP error: {e}")))?;
        let body = response.bytes().await
            .map_err(|e| r_data_core_core::error::Error::Api(format!("Failed to read response body: {e}")))?;

        Ok(Box::new(stream::iter(vec![Ok(body)])))
    }

    /// # Errors
    /// Returns an error if the configuration is invalid.
    fn validate(&self, config: &serde_json::Value) -> r_data_core_core::error::Result<()> {
        let uri = config
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| r_data_core_core::error::Error::Config("URI source requires 'uri' in config".to_string()))?;

        // Basic URI validation
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            return Err(r_data_core_core::error::Error::Validation("URI must start with http:// or https://".to_string()));
        }

        Ok(())
    }
}
