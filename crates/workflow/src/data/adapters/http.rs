#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::IpAddr;
use std::sync::{Arc, OnceLock};
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
        // Limit redirects to 5; the SSRF resolver re-validates every redirect
        // target because it fires on every DNS resolution reqwest performs.
        .redirect(reqwest::redirect::Policy::limited(5))
        // Closes finding #2 (DNS rebinding TOCTOU) and finding #3 (redirect
        // bypass): every address reqwest actually connects to — including
        // addresses for redirect targets — is filtered through `is_blocked_ip`
        // in strict mode.
        .dns_resolver(Arc::new(SsrfResolver))
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

/// Returns `true` for an IPv4 address that must never be reached from a
/// server-side fetch.
const fn is_blocked_ipv4(v4: std::net::Ipv4Addr) -> bool {
    let o = v4.octets();
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_broadcast()
        || v4.is_unspecified()
        // 0.0.0.0/8 — "this" network
        || o[0] == 0
        // CGNAT 100.64.0.0/10
        || (o[0] == 100 && (o[1] & 0xc0) == 64)
        // Benchmark addresses 198.18.0.0/15
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
}

/// Returns `true` for any IP address that must never be reachable from a
/// server-side fetch.
///
/// Closes finding #1: IPv4-mapped (`::ffff:a.b.c.d`) and IPv4-compatible
/// (`::a.b.c.d`) IPv6 addresses are unwrapped and subjected to the full V4
/// deny-list instead of falling through to the V6 path.
pub(super) const fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => {
            // IPv4-mapped  ::ffff:a.b.c.d  → treat as the embedded IPv4
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_blocked_ipv4(v4);
            }
            // IPv4-compatible  ::a.b.c.d  (deprecated, but still circulates)
            if let Some(v4) = v6.to_ipv4() {
                return is_blocked_ipv4(v4);
            }
            // Pure IPv6 checks
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique-local fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
        }
    }
}

/// reqwest DNS resolver that drops SSRF-blocked addresses in strict mode.
///
/// By plugging in at the DNS layer rather than in `guard_url`, every address
/// that reqwest **actually connects to** — including addresses selected after
/// auto-followed redirects — is validated against the block-list.  This closes
/// two separate attack windows:
///
/// * **DNS rebinding** (finding #2): the OS can return different answers between
///   the `guard_url` pre-flight and the real connect; the resolver fires on the
///   latter.
/// * **Redirect bypass** (finding #3): redirect targets are resolved through the
///   same resolver, so a `302 → http://169.254.169.254/…` hop is blocked before
///   the connection is established.
struct SsrfResolver;

impl Resolve for SsrfResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let strict = ssrf_strict_mode();
            let addrs = tokio::net::lookup_host(format!("{host}:0"))
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
            let filtered: Vec<std::net::SocketAddr> = addrs
                .filter(|a| !strict || !is_blocked_ip(a.ip()))
                .collect();
            if filtered.is_empty() {
                return Err::<Addrs, Box<dyn std::error::Error + Send + Sync>>(
                    format!("SSRF: all resolved addresses for {host} are blocked").into(),
                );
            }
            let iter: Addrs = Box::new(filtered.into_iter());
            Ok(iter)
        })
    }
}

/// Validate an outbound URL against SSRF policy. No-op outside production.
///
/// This is a fast pre-flight check for early failure and user-facing validation
/// feedback.  The authoritative enforcement is the `SsrfResolver` installed on
/// the shared HTTP client, which fires on every DNS resolution reqwest performs
/// (including redirect hops).
///
/// # Errors
/// Returns [`r_data_core_core::error::Error::Validation`] for non-http(s)
/// schemes, missing/unresolvable hosts, or hosts that resolve to a blocked IP
/// range (strict mode only).
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

    // --- Finding #1: IPv4-mapped / IPv4-compatible IPv6 bypass ---

    #[test]
    fn blocks_ipv4_mapped_loopback() {
        // ::ffff:127.0.0.1 — IPv4-mapped loopback
        assert!(is_blocked_ip("::ffff:127.0.0.1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn blocks_ipv4_mapped_private() {
        // ::ffff:10.0.0.1 — IPv4-mapped private
        assert!(is_blocked_ip("::ffff:10.0.0.1".parse::<IpAddr>().unwrap()));
    }

    // --- New IPv4 ranges ---

    #[test]
    fn blocks_cgnat() {
        // 100.64.0.0/10 — CGNAT
        assert!(is_blocked_ip("100.64.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_blocked_ip("100.127.255.255".parse::<IpAddr>().unwrap()));
        // Just outside the range — must NOT be blocked
        assert!(!is_blocked_ip("100.128.0.1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn blocks_benchmark_range() {
        // 198.18.0.0/15 — benchmark addresses
        assert!(is_blocked_ip("198.18.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_blocked_ip("198.19.255.255".parse::<IpAddr>().unwrap()));
        // Just outside — must NOT be blocked
        assert!(!is_blocked_ip("198.20.0.1".parse::<IpAddr>().unwrap()));
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
