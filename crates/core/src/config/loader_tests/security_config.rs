//! `SecurityConfig::from_env` — defaults, overrides, clamping and malformed input.

use super::{EnvGuard, ENV_MUTEX};
use crate::config::security::{
    SecurityConfig, DEFAULT_LOCKOUT_DURATION_SECS, DEFAULT_MAX_FAILED_ATTEMPTS,
    DEFAULT_RATE_LIMIT_MAX_ATTEMPTS, DEFAULT_RATE_LIMIT_WINDOW_SECS,
};

/// Every knob unset, so `from_env` must fall back to the documented defaults.
fn cleared() -> Vec<(&'static str, Option<&'static str>)> {
    vec![
        ("TRUSTED_PROXIES", None),
        ("LOGIN_MAX_FAILED_ATTEMPTS", None),
        ("LOGIN_LOCKOUT_DURATION_SECS", None),
        ("LOGIN_RATE_LIMIT_MAX_ATTEMPTS", None),
        ("LOGIN_RATE_LIMIT_WINDOW_SECS", None),
    ]
}

#[test]
fn unset_environment_yields_the_documented_defaults() {
    let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let _env = EnvGuard::new(&cleared());

    let cfg = SecurityConfig::from_env();
    assert_eq!(cfg.max_failed_attempts, DEFAULT_MAX_FAILED_ATTEMPTS);
    assert_eq!(cfg.lockout_duration_secs, DEFAULT_LOCKOUT_DURATION_SECS);
    assert_eq!(cfg.rate_limit_max_attempts, DEFAULT_RATE_LIMIT_MAX_ATTEMPTS);
    assert_eq!(cfg.rate_limit_window_secs, DEFAULT_RATE_LIMIT_WINDOW_SECS);
    assert!(
        cfg.trusted_proxies.is_empty(),
        "nothing is trusted by default"
    );
}

#[test]
fn explicit_values_are_read() {
    let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut overrides = cleared();
    overrides.push(("LOGIN_MAX_FAILED_ATTEMPTS", Some("3")));
    overrides.push(("LOGIN_LOCKOUT_DURATION_SECS", Some("60")));
    overrides.push(("LOGIN_RATE_LIMIT_MAX_ATTEMPTS", Some("25")));
    overrides.push(("LOGIN_RATE_LIMIT_WINDOW_SECS", Some("120")));
    let _env = EnvGuard::new(&overrides);

    let cfg = SecurityConfig::from_env();
    assert_eq!(cfg.max_failed_attempts, 3);
    assert_eq!(cfg.lockout_duration_secs, 60);
    assert_eq!(cfg.rate_limit_max_attempts, 25);
    assert_eq!(cfg.rate_limit_window_secs, 120);
}

#[test]
fn malformed_values_fall_back_to_defaults() {
    let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut overrides = cleared();
    overrides.push(("LOGIN_MAX_FAILED_ATTEMPTS", Some("not-a-number")));
    overrides.push(("LOGIN_RATE_LIMIT_WINDOW_SECS", Some("")));
    let _env = EnvGuard::new(&overrides);

    let cfg = SecurityConfig::from_env();
    assert_eq!(cfg.max_failed_attempts, DEFAULT_MAX_FAILED_ATTEMPTS);
    assert_eq!(cfg.rate_limit_window_secs, DEFAULT_RATE_LIMIT_WINDOW_SECS);
}

#[test]
fn whitespace_around_numbers_is_tolerated() {
    let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut overrides = cleared();
    overrides.push(("LOGIN_MAX_FAILED_ATTEMPTS", Some("  7  ")));
    let _env = EnvGuard::new(&overrides);

    assert_eq!(SecurityConfig::from_env().max_failed_attempts, 7);
}

/// A zero or negative threshold would lock every account on the first attempt,
/// and a zero window would make the limiter meaningless — both are clamped.
#[test]
fn nonsensical_values_are_clamped_to_a_safe_minimum() {
    let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut overrides = cleared();
    overrides.push(("LOGIN_MAX_FAILED_ATTEMPTS", Some("0")));
    overrides.push(("LOGIN_RATE_LIMIT_MAX_ATTEMPTS", Some("0")));
    overrides.push(("LOGIN_RATE_LIMIT_WINDOW_SECS", Some("0")));
    overrides.push(("LOGIN_LOCKOUT_DURATION_SECS", Some("-5")));
    let _env = EnvGuard::new(&overrides);

    let cfg = SecurityConfig::from_env();
    assert_eq!(cfg.max_failed_attempts, 1);
    assert_eq!(cfg.rate_limit_max_attempts, 1);
    assert_eq!(cfg.rate_limit_window_secs, 1);
    assert_eq!(cfg.lockout_duration_secs, 0, "negative means no expiry");
    assert!(!cfg.lock_expires());
}

#[test]
fn trusted_proxies_are_parsed_from_a_comma_list() {
    let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut overrides = cleared();
    overrides.push(("TRUSTED_PROXIES", Some("172.16.0.0/12, 10.1.2.3")));
    let _env = EnvGuard::new(&overrides);

    let cfg = SecurityConfig::from_env();
    assert!(cfg
        .trusted_proxies
        .is_trusted("172.18.0.9".parse().unwrap()));
    assert!(cfg.trusted_proxies.is_trusted("10.1.2.3".parse().unwrap()));
    assert!(!cfg.trusted_proxies.is_trusted("10.1.2.4".parse().unwrap()));
}

#[test]
fn a_malformed_proxy_entry_does_not_discard_the_valid_ones() {
    let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let mut overrides = cleared();
    overrides.push(("TRUSTED_PROXIES", Some("nonsense,,10.0.0.0/8")));
    let _env = EnvGuard::new(&overrides);

    let cfg = SecurityConfig::from_env();
    assert!(cfg.trusted_proxies.is_trusted("10.9.9.9".parse().unwrap()));
    assert!(!cfg.trusted_proxies.is_trusted("192.0.2.1".parse().unwrap()));
}
