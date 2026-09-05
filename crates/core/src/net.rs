#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! IP matching helpers used to identify trusted reverse proxies.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Normalise IPv4-mapped IPv6 (`::ffff:a.b.c.d`) to plain IPv4 so a v4 rule
/// matches regardless of the socket family the proxy connected over.
#[must_use]
pub fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        IpAddr::V4(_) => ip,
    }
}

fn mask(addr: IpAddr, prefix_len: u8) -> IpAddr {
    match addr {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4);
            let masked = if prefix_len == 0 {
                0
            } else {
                bits & (u32::MAX << (32 - prefix_len))
            };
            IpAddr::V4(Ipv4Addr::from(masked))
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6);
            let masked = if prefix_len == 0 {
                0
            } else {
                bits & (u128::MAX << (128 - prefix_len))
            };
            IpAddr::V6(Ipv6Addr::from(masked))
        }
    }
}

/// A single address or CIDR block, e.g. `10.0.0.0/8`, `::1`, `192.168.1.5`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpCidr {
    network: IpAddr,
    prefix_len: u8,
}

impl IpCidr {
    /// Parse an address or CIDR block. Returns `None` for malformed input.
    #[must_use]
    pub fn parse(entry: &str) -> Option<Self> {
        let entry = entry.trim();
        let (addr_part, prefix_part) = entry
            .split_once('/')
            .map_or((entry, None), |(a, p)| (a, Some(p)));

        let addr = normalize_ip(addr_part.trim().parse::<IpAddr>().ok()?);
        let max_prefix = if addr.is_ipv4() { 32 } else { 128 };
        let prefix_len = match prefix_part {
            Some(p) => p.trim().parse::<u8>().ok().filter(|n| *n <= max_prefix)?,
            None => max_prefix,
        };

        Some(Self {
            network: mask(addr, prefix_len),
            prefix_len,
        })
    }

    /// Whether `ip` falls inside this block.
    #[must_use]
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = normalize_ip(ip);
        ip.is_ipv4() == self.network.is_ipv4() && mask(ip, self.prefix_len) == self.network
    }
}

/// The set of reverse proxies whose `X-Forwarded-For` header may be trusted.
///
/// Empty by default: with no configured proxy the peer address is the only
/// thing we believe, which is the safe stance for a directly exposed server.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies(Vec<IpCidr>);

impl TrustedProxies {
    /// Parse a comma-separated list. Malformed entries are logged and skipped
    /// rather than failing startup.
    #[must_use]
    pub fn parse(list: &str) -> Self {
        let entries = list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| {
                let parsed = IpCidr::parse(s);
                if parsed.is_none() {
                    log::warn!("Ignoring malformed TRUSTED_PROXIES entry: {s}");
                }
                parsed
            })
            .collect();
        Self(entries)
    }

    /// Whether `ip` belongs to a configured proxy.
    #[must_use]
    pub fn is_trusted(&self, ip: IpAddr) -> bool {
        self.0.iter().any(|cidr| cidr.contains(ip))
    }

    /// Whether any proxy is configured at all.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{IpCidr, TrustedProxies};
    use std::net::IpAddr;

    fn ip(s: &str) -> IpAddr {
        s.parse()
            .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
    }

    #[test]
    fn bare_address_matches_only_itself() {
        let cidr = IpCidr::parse("192.168.1.5").unwrap_or_else(|| unreachable!());
        assert!(cidr.contains(ip("192.168.1.5")));
        assert!(!cidr.contains(ip("192.168.1.6")));
    }

    #[test]
    fn cidr_block_matches_range() {
        let cidr = IpCidr::parse("172.16.0.0/12").unwrap_or_else(|| unreachable!());
        assert!(cidr.contains(ip("172.18.0.5")));
        assert!(cidr.contains(ip("172.31.255.255")));
        assert!(!cidr.contains(ip("172.32.0.1")));
        assert!(!cidr.contains(ip("10.0.0.1")));
    }

    #[test]
    fn zero_prefix_matches_every_address_of_its_family() {
        let cidr = IpCidr::parse("0.0.0.0/0").unwrap_or_else(|| unreachable!());
        assert!(cidr.contains(ip("8.8.8.8")));
        assert!(!cidr.contains(ip("2001:db8::1")));
    }

    #[test]
    fn ipv4_mapped_ipv6_is_normalised() {
        let cidr = IpCidr::parse("172.16.0.0/12").unwrap_or_else(|| unreachable!());
        assert!(cidr.contains(ip("::ffff:172.18.0.5")));
    }

    #[test]
    fn ipv6_blocks_are_supported() {
        let cidr = IpCidr::parse("2001:db8::/32").unwrap_or_else(|| unreachable!());
        assert!(cidr.contains(ip("2001:db8::1")));
        assert!(!cidr.contains(ip("2001:db9::1")));
    }

    #[test]
    fn malformed_entries_are_rejected() {
        assert!(IpCidr::parse("not-an-ip").is_none());
        assert!(IpCidr::parse("10.0.0.0/33").is_none());
        assert!(IpCidr::parse("").is_none());
    }

    #[test]
    fn proxy_list_skips_malformed_entries() {
        let proxies = TrustedProxies::parse("10.0.0.0/8, nonsense, ::1");
        assert!(proxies.is_trusted(ip("10.1.2.3")));
        assert!(proxies.is_trusted(ip("::1")));
        assert!(!proxies.is_trusted(ip("192.168.0.1")));
    }

    #[test]
    fn empty_list_trusts_nothing() {
        let proxies = TrustedProxies::parse("");
        assert!(proxies.is_empty());
        assert!(!proxies.is_trusted(ip("10.0.0.1")));
    }
}
