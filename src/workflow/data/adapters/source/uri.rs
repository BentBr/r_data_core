use super::{DataSource, SourceContext};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;

/// URI-based data source (HTTP/HTTPS)
pub struct UriSource;

impl UriSource {
    pub fn new() -> Self {
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
    ) -> Result<Box<dyn Stream<Item = Result<Bytes>> + Unpin + Send>> {
        let uri = ctx
            .config
            .get("uri")
            .and_then(|v| v.as_str())
            .context("URI source requires 'uri' in config")?;

        let client = reqwest::Client::new();
        let mut request = client.get(uri);

        // Apply authentication if provided
        if let Some(auth) = &ctx.auth {
            request = auth.apply_to_request(request)?;
        }

        let response = request.send().await?.error_for_status()?;
        let body = response.bytes().await?;

        use futures::stream;
        Ok(Box::new(stream::iter(vec![Ok(body)])))
    }

    fn validate(&self, config: &serde_json::Value) -> Result<()> {
        let uri = config
            .get("uri")
            .and_then(|v| v.as_str())
            .context("URI source requires 'uri' in config")?;

        // Basic URI validation
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            anyhow::bail!("URI must start with http:// or https://");
        }

        Ok(())
    }
}
