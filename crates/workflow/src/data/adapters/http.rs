#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::net::IpAddr;
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

/// Returns true when the environment is production (strict SSRF mode).
fn ssrf_strict_mode() -> bool {
    std::env::var("APP_ENV").is_ok_and(|e| e.eq_ignore_ascii_case("production"))
}

/// Hosts explicitly allowed even in strict mode (comma-separated `SSRF_ALLOWED_HOSTS`).
fn ssrf_allowlist() -> Vec<String> {
    std::env::var("SSRF_ALLOWED_HOSTS")
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Returns true for IPs that must never be reachable from a server-side fetch.
const fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.octets()[0] == 0
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique-local fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
        }
    }
}

/// Validate an outbound URL against SSRF policy. No-op outside production.
///
/// # Errors
/// Returns [`r_data_core_core::error::Error::Validation`] for non-http(s) schemes, missing/unresolvable
/// hosts, or hosts that resolve to a blocked IP range (strict mode only).
pub(super) async fn guard_url(uri: &str) -> r_data_core_core::error::Result<()> {
    let url = reqwest::Url::parse(uri)
        .map_err(|e| r_data_core_core::error::Error::Validation(format!("Invalid URI: {e}")))?;
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "URI scheme not allowed: {other}"
            )));
        }
    }

    if !ssrf_strict_mode() {
        return Ok(());
    }

    let host = url
        .host_str()
        .ok_or_else(|| r_data_core_core::error::Error::Validation("URI has no host".to_string()))?;

    if ssrf_allowlist().contains(&host.to_lowercase()) {
        return Ok(());
    }

    let port = url.port_or_known_default().unwrap_or(80);
    let addrs = tokio::net::lookup_host((host, port)).await.map_err(|e| {
        r_data_core_core::error::Error::Validation(format!("Cannot resolve host {host}: {e}"))
    })?;

    for addr in addrs {
        if is_blocked_ip(addr.ip()) {
            return Err(r_data_core_core::error::Error::Validation(format!(
                "URI resolves to a blocked address: {host}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod ssrf_tests {
    use super::{guard_url, is_blocked_ip};
    use std::net::IpAddr;

    #[test]
    fn blocks_loopback_private_and_metadata() {
        assert!(is_blocked_ip("127.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_blocked_ip("10.0.0.5".parse::<IpAddr>().unwrap()));
        assert!(is_blocked_ip("192.168.1.1".parse::<IpAddr>().unwrap()));
        assert!(is_blocked_ip("169.254.169.254".parse::<IpAddr>().unwrap()));
        assert!(is_blocked_ip("::1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn allows_public_ip() {
        assert!(!is_blocked_ip("8.8.8.8".parse::<IpAddr>().unwrap()));
    }

    #[tokio::test]
    async fn non_http_scheme_rejected() {
        assert!(guard_url("file:///etc/passwd").await.is_err());
        assert!(guard_url("ftp://example.com").await.is_err());
    }

    #[tokio::test]
    async fn non_production_allows_loopback() {
        // APP_ENV unset in tests => non-strict => loopback allowed (scheme still checked).
        assert!(guard_url("http://127.0.0.1:8080/data").await.is_ok());
    }
}
