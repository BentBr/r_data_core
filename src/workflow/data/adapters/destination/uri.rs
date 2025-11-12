use super::{DataDestination, DestinationContext, HttpMethod};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;

/// URI-based data destination (HTTP/HTTPS)
pub struct UriDestination;

impl UriDestination {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DataDestination for UriDestination {
    fn destination_type(&self) -> &'static str {
        "uri"
    }

    async fn push(&self, ctx: &DestinationContext, data: Bytes) -> Result<()> {
        let uri = ctx
            .config
            .get("uri")
            .and_then(|v| v.as_str())
            .context("URI destination requires 'uri' in config")?;

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
            request = auth.apply_to_request(request)?;
        }

        // Add body for methods that require it
        if method.requires_body() {
            request = request.body(data);
        }

        let response = request.send().await?.error_for_status()?;
        let _body = response.bytes().await?;

        Ok(())
    }

    fn validate(&self, config: &serde_json::Value) -> Result<()> {
        let uri = config
            .get("uri")
            .and_then(|v| v.as_str())
            .context("URI destination requires 'uri' in config")?;

        // Basic URI validation
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            anyhow::bail!("URI must start with http:// or https://");
        }

        Ok(())
    }
}

