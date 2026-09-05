#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Determine the address to attribute a request to.
//!
//! Behind a reverse proxy every request has the proxy's peer address, so a
//! per-IP limit keyed on it throttles the whole world at once. `X-Forwarded-For`
//! carries the real client but is attacker-controlled, so it is only believed
//! when the immediate peer is a configured trusted proxy.

use std::net::IpAddr;

use actix_web::HttpRequest;
use r_data_core_core::net::{normalize_ip, TrustedProxies};

const FORWARDED_FOR: &str = "X-Forwarded-For";

/// The address a request is attributed to, or `None` for a peerless request
/// (in practice only synthetic test requests).
#[must_use]
pub fn client_ip(req: &HttpRequest, trusted: &TrustedProxies) -> Option<IpAddr> {
    let peer = normalize_ip(req.peer_addr()?.ip());

    if !trusted.is_trusted(peer) {
        // A direct client cannot talk us into believing a forged header.
        return Some(peer);
    }

    let forwarded = req
        .headers()
        .get(FORWARDED_FOR)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    // Walk right to left: entries appended by our own proxies are trusted, the
    // first untrusted one is the closest address we can still believe.
    forwarded
        .rsplit(',')
        .filter_map(|entry| entry.trim().parse::<IpAddr>().ok())
        .map(normalize_ip)
        .find(|ip| !trusted.is_trusted(*ip))
        .or(Some(peer))
}

/// Cache-key fragment for a request's client address.
///
/// Requests with no peer address share the `unknown` bucket: lumping them
/// together throttles more than strictly necessary, but the alternative — no
/// limit at all — would be a bypass.
#[must_use]
pub fn client_ip_key(req: &HttpRequest, trusted: &TrustedProxies) -> String {
    client_ip(req, trusted).map_or_else(|| "unknown".to_string(), |ip| ip.to_string())
}

#[cfg(test)]
mod tests {
    use super::{client_ip, client_ip_key};
    use actix_web::test::TestRequest;
    use r_data_core_core::net::TrustedProxies;

    fn proxies(list: &str) -> TrustedProxies {
        TrustedProxies::parse(list)
    }

    #[test]
    fn direct_client_uses_the_peer_address() {
        let req = TestRequest::default()
            .peer_addr(
                "203.0.113.9:5000"
                    .parse()
                    .unwrap_or_else(|_| unreachable!()),
            )
            .insert_header(("X-Forwarded-For", "1.2.3.4"))
            .to_http_request();

        // Header is ignored: the peer is not a configured proxy.
        assert_eq!(client_ip_key(&req, &proxies("10.0.0.0/8")), "203.0.113.9");
    }

    #[test]
    fn trusted_proxy_reveals_the_forwarded_client() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "203.0.113.9"))
            .to_http_request();

        assert_eq!(
            client_ip_key(&req, &proxies("172.16.0.0/12")),
            "203.0.113.9"
        );
    }

    #[test]
    fn spoofed_entries_left_of_the_real_client_are_ignored() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "9.9.9.9, 203.0.113.9"))
            .to_http_request();

        // The client may prepend anything; only the rightmost untrusted hop counts.
        assert_eq!(
            client_ip_key(&req, &proxies("172.16.0.0/12")),
            "203.0.113.9"
        );
    }

    #[test]
    fn chained_trusted_proxies_are_skipped() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "203.0.113.9, 172.18.0.7"))
            .to_http_request();

        assert_eq!(
            client_ip_key(&req, &proxies("172.16.0.0/12")),
            "203.0.113.9"
        );
    }

    #[test]
    fn trusted_proxy_without_a_header_falls_back_to_the_peer() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .to_http_request();

        assert_eq!(client_ip_key(&req, &proxies("172.16.0.0/12")), "172.18.0.2");
    }

    #[test]
    fn malformed_header_entries_are_skipped() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "203.0.113.9, junk"))
            .to_http_request();

        assert_eq!(
            client_ip_key(&req, &proxies("172.16.0.0/12")),
            "203.0.113.9"
        );
    }

    #[test]
    fn no_peer_address_yields_the_shared_bucket() {
        let req = TestRequest::default().to_http_request();

        assert!(client_ip(&req, &proxies("10.0.0.0/8")).is_none());
        assert_eq!(client_ip_key(&req, &proxies("10.0.0.0/8")), "unknown");
    }

    #[test]
    fn ipv6_peers_and_proxies_are_supported() {
        let req = TestRequest::default()
            .peer_addr(
                "[2001:db8::2]:5000"
                    .parse()
                    .unwrap_or_else(|_| unreachable!()),
            )
            // Outside the proxy block, so this is the real client.
            .insert_header(("X-Forwarded-For", "2001:db9::9"))
            .to_http_request();

        assert_eq!(
            client_ip_key(&req, &proxies("2001:db8::/32")),
            "2001:db9::9"
        );
    }

    #[test]
    fn an_ipv4_mapped_peer_matches_an_ipv4_proxy_rule() {
        // Dual-stack sockets surface the proxy as ::ffff:172.18.0.2.
        let req = TestRequest::default()
            .peer_addr(
                "[::ffff:172.18.0.2]:5000"
                    .parse()
                    .unwrap_or_else(|_| unreachable!()),
            )
            .insert_header(("X-Forwarded-For", "203.0.113.9"))
            .to_http_request();

        assert_eq!(
            client_ip_key(&req, &proxies("172.16.0.0/12")),
            "203.0.113.9"
        );
    }

    #[test]
    fn an_ipv4_mapped_forwarded_entry_is_normalised() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "::ffff:203.0.113.9"))
            .to_http_request();

        assert_eq!(
            client_ip_key(&req, &proxies("172.16.0.0/12")),
            "203.0.113.9"
        );
    }

    #[test]
    fn extra_whitespace_between_entries_is_tolerated() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "  9.9.9.9 ,   203.0.113.9   "))
            .to_http_request();

        assert_eq!(
            client_ip_key(&req, &proxies("172.16.0.0/12")),
            "203.0.113.9"
        );
    }

    #[test]
    fn an_empty_header_falls_back_to_the_peer() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", ""))
            .to_http_request();

        assert_eq!(client_ip_key(&req, &proxies("172.16.0.0/12")), "172.18.0.2");
    }

    /// Entries carrying a port do not parse as bare addresses and are skipped;
    /// the result stays conservative rather than becoming attacker-controlled.
    #[test]
    fn entries_with_a_port_are_skipped_not_trusted() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "203.0.113.9:1234"))
            .to_http_request();

        assert_eq!(client_ip_key(&req, &proxies("172.16.0.0/12")), "172.18.0.2");
    }

    /// Every hop is one of ours, so there is no client address to recover and
    /// the peer is the best available answer.
    #[test]
    fn a_header_containing_only_trusted_hops_falls_back_to_the_peer() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "172.18.0.5, 172.18.0.7"))
            .to_http_request();

        assert_eq!(client_ip_key(&req, &proxies("172.16.0.0/12")), "172.18.0.2");
    }

    #[test]
    fn client_ip_returns_the_parsed_address_not_only_a_key() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "203.0.113.9"))
            .to_http_request();

        assert_eq!(
            client_ip(&req, &proxies("172.16.0.0/12")),
            "203.0.113.9".parse().ok()
        );
    }

    #[test]
    fn no_configured_proxy_always_uses_the_peer() {
        let req = TestRequest::default()
            .peer_addr("172.18.0.2:5000".parse().unwrap_or_else(|_| unreachable!()))
            .insert_header(("X-Forwarded-For", "203.0.113.9"))
            .to_http_request();

        assert_eq!(client_ip_key(&req, &proxies("")), "172.18.0.2");
    }
}
