#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Login-hardening knobs (trusted proxies, lockout, per-IP rate limit).
//!
//! Kept out of [`ApiConfig`](crate::config::ApiConfig) and read straight from
//! the environment so the values are available to request handlers without
//! threading a new field through every `ApiState` implementation.

use std::env;
use std::sync::OnceLock;

use crate::net::TrustedProxies;

/// Default number of failed passwords before an account is locked.
pub const DEFAULT_MAX_FAILED_ATTEMPTS: i32 = 5;
/// Default lockout duration, seconds (15 minutes).
pub const DEFAULT_LOCKOUT_DURATION_SECS: i64 = 900;
/// Default failed logins per client IP per window.
pub const DEFAULT_RATE_LIMIT_MAX_ATTEMPTS: u32 = 10;
/// Default rate-limit window, seconds (15 minutes).
pub const DEFAULT_RATE_LIMIT_WINDOW_SECS: u64 = 900;

static GLOBAL: OnceLock<SecurityConfig> = OnceLock::new();

/// Authentication-hardening settings.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Reverse proxies whose `X-Forwarded-For` header may be believed.
    pub trusted_proxies: TrustedProxies,
    /// Failed passwords before the account is locked.
    pub max_failed_attempts: i32,
    /// How long a lock lasts before it expires on its own, seconds.
    pub lockout_duration_secs: i64,
    /// Failed logins per client IP per window.
    pub rate_limit_max_attempts: u32,
    /// Rate-limit window, seconds.
    pub rate_limit_window_secs: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            trusted_proxies: TrustedProxies::default(),
            max_failed_attempts: DEFAULT_MAX_FAILED_ATTEMPTS,
            lockout_duration_secs: DEFAULT_LOCKOUT_DURATION_SECS,
            rate_limit_max_attempts: DEFAULT_RATE_LIMIT_MAX_ATTEMPTS,
            rate_limit_window_secs: DEFAULT_RATE_LIMIT_WINDOW_SECS,
        }
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<T>().ok())
        .unwrap_or(default)
}

impl SecurityConfig {
    /// Read the settings from the environment, falling back to the defaults.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            trusted_proxies: TrustedProxies::parse(
                &env::var("TRUSTED_PROXIES").unwrap_or_default(),
            ),
            max_failed_attempts: parse_env(
                "LOGIN_MAX_FAILED_ATTEMPTS",
                DEFAULT_MAX_FAILED_ATTEMPTS,
            )
            .max(1),
            lockout_duration_secs: parse_env(
                "LOGIN_LOCKOUT_DURATION_SECS",
                DEFAULT_LOCKOUT_DURATION_SECS,
            )
            .max(0),
            rate_limit_max_attempts: parse_env(
                "LOGIN_RATE_LIMIT_MAX_ATTEMPTS",
                DEFAULT_RATE_LIMIT_MAX_ATTEMPTS,
            )
            .max(1),
            rate_limit_window_secs: parse_env(
                "LOGIN_RATE_LIMIT_WINDOW_SECS",
                DEFAULT_RATE_LIMIT_WINDOW_SECS,
            )
            .max(1),
        }
    }

    /// Process-wide settings, initialised from the environment on first use.
    #[must_use]
    pub fn global() -> &'static Self {
        GLOBAL.get_or_init(Self::from_env)
    }

    /// A lock older than [`Self::lockout_duration_secs`] has expired.
    /// A duration of `0` means locks never expire on their own.
    #[must_use]
    pub const fn lock_expires(&self) -> bool {
        self.lockout_duration_secs > 0
    }
}

#[cfg(test)]
mod tests {
    use super::{SecurityConfig, DEFAULT_MAX_FAILED_ATTEMPTS};

    #[test]
    fn defaults_are_sane() {
        let cfg = SecurityConfig::default();
        assert_eq!(cfg.max_failed_attempts, DEFAULT_MAX_FAILED_ATTEMPTS);
        assert!(cfg.trusted_proxies.is_empty());
        assert!(cfg.lock_expires());
    }

    #[test]
    fn zero_lockout_duration_disables_expiry() {
        let cfg = SecurityConfig {
            lockout_duration_secs: 0,
            ..SecurityConfig::default()
        };
        assert!(!cfg.lock_expires());
    }
}
