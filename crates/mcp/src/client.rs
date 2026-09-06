#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Typed access to the `RDataCore` admin API.

pub mod error;

pub use error::ClientError;

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::auth::{AuthBackend, CallerContext};
use crate::config::Config;

/// `RDataCore`'s standard response envelope, from
/// `crates/api/src/response/mod.rs`.
///
/// `data` is **optional**: a 2xx carrying a null payload is possible, and the
/// client must say so clearly rather than unwrap.
#[derive(Debug, Deserialize)]
pub struct Envelope<T> {
    #[serde(default = "Option::default")]
    pub data: Option<T>,
    #[serde(default)]
    pub message: String,
}

impl<T> Envelope<T> {
    /// # Errors
    /// Returns `ClientError::Decode` when a successful response carried no data.
    pub fn require_data(self) -> Result<T, ClientError> {
        let message = self.message;
        self.data.ok_or_else(|| ClientError::Decode {
            message: format!("response contained no data (message: {message})"),
        })
    }
}

/// HTTP client for the `RDataCore` admin API.
pub struct RdcClient {
    http: reqwest::Client,
    base_url: Url,
    auth: Arc<dyn AuthBackend>,
}

impl RdcClient {
    /// # Errors
    /// Returns `ClientError` if the underlying HTTP client cannot be built.
    pub fn new(config: &Config, auth: Arc<dyn AuthBackend>) -> Result<Self, ClientError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| ClientError::Decode {
                message: e.to_string(),
            })?;
        Ok(Self {
            http,
            base_url: config.base_url.clone(),
            auth,
        })
    }

    /// # Errors
    /// Returns `ClientError` on transport failure or a non-2xx response.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        ctx: &CallerContext,
    ) -> Result<T, ClientError> {
        self.send::<(), T>(reqwest::Method::GET, path, None, ctx)
            .await
    }

    /// # Errors
    /// As [`Self::get_json`].
    pub async fn post_json<B: Serialize + Sync, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        ctx: &CallerContext,
    ) -> Result<T, ClientError> {
        self.send(reqwest::Method::POST, path, Some(body), ctx)
            .await
    }

    /// # Errors
    /// As [`Self::get_json`].
    pub async fn put_json<B: Serialize + Sync, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        ctx: &CallerContext,
    ) -> Result<T, ClientError> {
        self.send(reqwest::Method::PUT, path, Some(body), ctx).await
    }

    /// Build an absolute URL for an API path.
    ///
    /// `Url::join` treats a leading `/` as replacing the base's whole path, so
    /// a base of `https://host/rdc` would silently lose its prefix. Joining the
    /// trimmed pieces by hand keeps a base path intact.
    fn url_for(&self, path: &str) -> Result<Url, ClientError> {
        let base = self.base_url.as_str().trim_end_matches('/');
        let path = path.trim_start_matches('/');
        Url::parse(&format!("{base}/{path}")).map_err(|e| ClientError::Decode {
            message: format!("could not build a URL for '{path}': {e}"),
        })
    }

    async fn send<B: Serialize + Sync, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&B>,
        ctx: &CallerContext,
    ) -> Result<T, ClientError> {
        let url = self.url_for(path)?;

        let mut req = self.http.request(method, url.clone());
        for (name, value) in self
            .auth
            .outbound_headers(ctx)
            .await
            .map_err(|_| ClientError::Unauthorized)?
        {
            req = req.header(name, value);
        }
        if let Some(body) = body {
            req = req.json(body);
        }

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                ClientError::Timeout
            } else {
                ClientError::Unreachable {
                    url: url.to_string(),
                }
            }
        })?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| ClientError::Decode {
            message: e.to_string(),
        })?;

        if !status.is_success() {
            let parsed = serde_json::from_str::<serde_json::Value>(&text)
                .unwrap_or_else(|_| serde_json::json!({ "message": text }));
            return Err(ClientError::from_api_body(
                status.as_u16(),
                &parsed,
                url.as_str(),
            ));
        }

        serde_json::from_str::<T>(&text).map_err(|e| ClientError::Decode {
            message: format!(
                "{e} (body starts: {})",
                text.chars().take(200).collect::<String>()
            ),
        })
    }
}

#[cfg(test)]
mod tests;
