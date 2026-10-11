#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Per-client-IP throttling for the unauthenticated auth endpoints.

use actix_web::HttpRequest;
use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::security::{
    API_KEY_EXCHANGE_RATE_LIMIT_MAX, API_KEY_EXCHANGE_RATE_LIMIT_WINDOW_SECS,
    OIDC_START_RATE_LIMIT_MAX, OIDC_START_RATE_LIMIT_WINDOW_SECS,
};
use r_data_core_core::config::SecurityConfig;

use crate::client_ip::client_ip_key;

/// Which unauthenticated endpoint a counter belongs to. Separate buckets keep
/// registration probes from throttling legitimate logins and vice versa.
#[derive(Debug, Clone, Copy)]
pub enum Bucket {
    /// Failed admin logins.
    Login,
    /// Unauthenticated registration requests.
    Register,
    /// Single-sign-on starts: every one costs a provider round trip.
    OidcStart,
    /// API-key → token exchanges.
    ApiKeyExchange,
}

impl Bucket {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Login => "login_rl",
            Self::Register => "register_rl",
            Self::OidcStart => "oidc_start_rl",
            Self::ApiKeyExchange => "api_key_exchange_rl",
        }
    }

    /// `(max attempts, window seconds)` for this bucket.
    ///
    /// Login and registration count failures (or anonymous requests) and use
    /// the configured login limits. Single-sign-on starts and API-key
    /// exchanges count every call, successes included, so they get their own,
    /// higher thresholds — otherwise the eleventh colleague behind one NAT
    /// would be refused.
    #[must_use]
    pub const fn limits(self, cfg: &SecurityConfig) -> (u32, u64) {
        match self {
            Self::Login | Self::Register => {
                (cfg.rate_limit_max_attempts, cfg.rate_limit_window_secs)
            }
            Self::OidcStart => (OIDC_START_RATE_LIMIT_MAX, OIDC_START_RATE_LIMIT_WINDOW_SECS),
            Self::ApiKeyExchange => (
                API_KEY_EXCHANGE_RATE_LIMIT_MAX,
                API_KEY_EXCHANGE_RATE_LIMIT_WINDOW_SECS,
            ),
        }
    }
}

/// Derive the rate-limit cache key for a request's client IP.
#[must_use]
pub fn rate_limit_key(req: &HttpRequest, bucket: Bucket) -> String {
    let ip = client_ip_key(req, &SecurityConfig::global().trusted_proxies);
    format!("{}:{ip}", bucket.prefix())
}

/// Returns `true` when `attempts` has reached this bucket's limit.
#[must_use]
pub fn is_rate_limited(bucket: Bucket, attempts: u32) -> bool {
    attempts >= bucket.limits(SecurityConfig::global()).0
}

/// Whether this request should be rejected before doing any real work.
pub async fn is_over_limit(cache: &CacheManager, bucket: Bucket, key: &str) -> bool {
    let attempts = cache.get::<u32>(key).await.ok().flatten().unwrap_or(0);
    is_rate_limited(bucket, attempts)
}

/// Record one attempt against this key and return the new count.
///
/// Atomic, so parallel requests cannot undercount their way past the limit.
/// Best-effort: cache errors are ignored (account lockout is the durable
/// backstop).
pub async fn record_failure(cache: &CacheManager, bucket: Bucket, key: &str) -> u32 {
    let (_, window) = bucket.limits(SecurityConfig::global());
    cache.increment(key, window).await.unwrap_or(0)
}

/// Clear a counter after a successful login so a legitimate user is not
/// throttled by earlier mistakes.
pub async fn reset(cache: &CacheManager, key: &str) {
    let _ = cache.delete(key).await;
}

#[cfg(test)]
mod tests {
    use super::{is_rate_limited, Bucket};
    use r_data_core_core::config::SecurityConfig;

    #[test]
    fn under_limit_is_allowed() {
        let max = SecurityConfig::global().rate_limit_max_attempts;
        assert!(!is_rate_limited(Bucket::Login, 0));
        assert!(!is_rate_limited(Bucket::Login, max - 1));
    }

    #[test]
    fn at_or_over_limit_is_blocked() {
        let max = SecurityConfig::global().rate_limit_max_attempts;
        assert!(is_rate_limited(Bucket::Login, max));
        assert!(is_rate_limited(Bucket::Login, max + 5));
    }

    #[test]
    fn counting_buckets_have_their_own_higher_limits() {
        let cfg = SecurityConfig::default();
        let login = Bucket::Login.limits(&cfg);
        assert_eq!(
            login,
            (cfg.rate_limit_max_attempts, cfg.rate_limit_window_secs)
        );
        assert_eq!(Bucket::Register.limits(&cfg), login);
        assert_eq!(Bucket::OidcStart.limits(&cfg), (60, 60));
        assert_eq!(Bucket::ApiKeyExchange.limits(&cfg), (60, 60));
        // Eleven sign-ins from one NAT must not be refused.
        assert!(!is_rate_limited(Bucket::OidcStart, 11));
        assert!(!is_rate_limited(Bucket::ApiKeyExchange, 11));
    }

    #[test]
    fn buckets_do_not_share_a_counter() {
        let prefixes = [
            Bucket::Login.prefix(),
            Bucket::Register.prefix(),
            Bucket::OidcStart.prefix(),
            Bucket::ApiKeyExchange.prefix(),
        ];
        for (i, a) in prefixes.iter().enumerate() {
            for b in &prefixes[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
