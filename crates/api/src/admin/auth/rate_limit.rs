#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Per-client-IP throttling for the unauthenticated auth endpoints.

use actix_web::HttpRequest;
use r_data_core_core::cache::CacheManager;
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
}

impl Bucket {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Login => "login_rl",
            Self::Register => "register_rl",
        }
    }
}

/// Derive the rate-limit cache key for a request's client IP.
#[must_use]
pub fn rate_limit_key(req: &HttpRequest, bucket: Bucket) -> String {
    let ip = client_ip_key(req, &SecurityConfig::global().trusted_proxies);
    format!("{}:{ip}", bucket.prefix())
}

/// Returns `true` when `attempts` has reached the configured limit.
#[must_use]
pub fn is_rate_limited(attempts: u32) -> bool {
    attempts >= SecurityConfig::global().rate_limit_max_attempts
}

/// Whether this request should be rejected before doing any real work.
pub async fn is_over_limit(cache: &CacheManager, key: &str) -> bool {
    let attempts = cache.get::<u32>(key).await.ok().flatten().unwrap_or(0);
    is_rate_limited(attempts)
}

/// Record one attempt against this key and return the new count.
///
/// Atomic, so parallel requests cannot undercount their way past the limit.
/// Best-effort: cache errors are ignored (account lockout is the durable
/// backstop).
pub async fn record_failure(cache: &CacheManager, key: &str) -> u32 {
    let window = SecurityConfig::global().rate_limit_window_secs;
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
        assert!(!is_rate_limited(0));
        assert!(!is_rate_limited(max - 1));
    }

    #[test]
    fn at_or_over_limit_is_blocked() {
        let max = SecurityConfig::global().rate_limit_max_attempts;
        assert!(is_rate_limited(max));
        assert!(is_rate_limited(max + 5));
    }

    #[test]
    fn buckets_do_not_share_a_counter() {
        assert_ne!(Bucket::Login.prefix(), Bucket::Register.prefix());
    }
}
