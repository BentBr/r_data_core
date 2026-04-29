#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::OnceLock;
use std::time::Duration;

const URI_CONNECT_TIMEOUT_SECS: u64 = 5;
const URI_REQUEST_TIMEOUT_SECS: u64 = 30;
static URI_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub(super) fn uri_http_client() -> r_data_core_core::error::Result<&'static reqwest::Client> {
    if let Some(client) = URI_HTTP_CLIENT.get() {
        return Ok(client);
    }

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(URI_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(URI_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| {
            r_data_core_core::error::Error::Config(format!("Failed to create HTTP client: {e}"))
        })?;

    Ok(URI_HTTP_CLIENT.get_or_init(|| client))
}
