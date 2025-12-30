use super::{DataDestination, DestinationContext, HttpMethod};
use async_trait::async_trait;
use bytes::Bytes;

/// URI-based data destination (HTTP/HTTPS)
#[derive(Default)]
pub struct UriDestination;

impl UriDestination {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DataDestination for UriDestination {
    fn destination_type(&self) -> &'static str {
        "uri"
    }

    async fn push(
        &self,
        ctx: &DestinationContext,
        data: Bytes,
    ) -> r_data_core_core::error::Result<()> {
        let uri = ctx
            .config
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                r_data_core_core::error::Error::Config(
                    "URI destination requires 'uri' in config".to_string(),
                )
            })?;

        let method = ctx.method.unwrap_or(HttpMethod::Post);
        let client = reqwest::Client::new();

        let mut request = match method {
            HttpMethod::Get => client.get(uri),
            HttpMethod::Post => client.post(uri),
            HttpMethod::Put => client.put(uri),
            HttpMethod::Patch => client.patch(uri),
            HttpMethod::Delete => client.delete(uri),
            HttpMethod::Head => client.head(uri),
            HttpMethod::Options => client.request(reqwest::Method::OPTIONS, uri),
        };

        // Apply authentication if provided
        if let Some(auth) = &ctx.auth {
            request = auth
                .apply_to_request(request)
                .map_err(|e| r_data_core_core::error::Error::Api(e.to_string()))?;
        }

        // Add body for methods that require it
        if method.requires_body() {
            request = request.body(data);
        }

        let response = request.send().await.map_err(|e| {
            r_data_core_core::error::Error::Api(format!("Failed to send request: {e}"))
        })?;
        let response = response
            .error_for_status()
            .map_err(|e| r_data_core_core::error::Error::Api(format!("HTTP error: {e}")))?;
        let _body = response.bytes().await.map_err(|e| {
            r_data_core_core::error::Error::Api(format!("Failed to read response body: {e}"))
        })?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the configuration is invalid.
    fn validate(&self, config: &serde_json::Value) -> r_data_core_core::error::Result<()> {
        let uri = config.get("uri").and_then(|v| v.as_str()).ok_or_else(|| {
            r_data_core_core::error::Error::Config(
                "URI destination requires 'uri' in config".to_string(),
            )
        })?;

        // Basic URI validation
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            return Err(r_data_core_core::error::Error::Validation(
                "URI must start with http:// or https://".to_string(),
            ));
        }

        Ok(())
    }
}
