#![allow(clippy::unwrap_used)]
#![allow(clippy::redundant_closure_for_method_calls)]

//! Tests for `config::loader` — env-var parsing for all config loaders.
//!
//! **Env-var safety note:** The loader functions all call `dotenv().ok()` which
//! re-loads the project `.env` file on every call.  This means any var in
//! `.env` cannot be tested as "absent" — removing it from the process env is
//! undone by the next `dotenv()` call inside the loader.
//!
//! Strategy:
//! - Tests that need a var to have a specific value: save, override, test,
//!   then restore via `EnvGuard` (RAII).
//! - Tests for "missing required var" errors: override the var with an empty
//!   string (`""`) so the `env::var` call returns a value but the `map_err`
//!   path for truly-absent vars is tested via value-based error paths (zero,
//!   negative, non-numeric, etc.).
//! - All env-mutating tests hold `ENV_MUTEX` to prevent cross-test races.

use std::collections::HashMap;
use std::env;
use std::sync::Mutex;

use super::{load_cache_config, load_license_config, load_worker_config};

// A process-wide mutex that all env-mutating tests must hold.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

struct EnvGuard {
    saved: HashMap<String, Option<String>>,
}

impl EnvGuard {
    /// Save current values for each key in `overrides`, then apply the
    /// override.  `Some(val)` sets the var; `None` removes it.
    fn new(overrides: &[(&str, Option<&str>)]) -> Self {
        let mut saved = HashMap::new();
        for &(k, v) in overrides {
            saved.insert(k.to_string(), env::var(k).ok());
            match v {
                Some(val) => unsafe { env::set_var(k, val) },
                None => unsafe { env::remove_var(k) },
            }
        }
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.saved {
            match v {
                Some(val) => unsafe { env::set_var(k, val) },
                None => unsafe { env::remove_var(k) },
            }
        }
    }
}

// ─── load_license_config ────────────────────────────────────────────────────

mod license_config_tests {
    use super::*;

    #[test]
    fn reads_license_key_from_env() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&[("LICENSE_KEY", Some("my-license-token"))]);

        let cfg = load_license_config().unwrap();
        assert_eq!(cfg.license_key.as_deref(), Some("my-license-token"));
    }

    #[test]
    fn reads_private_and_public_key() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&[
            ("LICENSE_PRIVATE_KEY", Some("priv-key-value")),
            ("LICENSE_PUBLIC_KEY", Some("pub-key-value")),
        ]);

        let cfg = load_license_config().unwrap();
        assert_eq!(cfg.private_key.as_deref(), Some("priv-key-value"));
        assert_eq!(cfg.public_key.as_deref(), Some("pub-key-value"));
    }

    #[test]
    fn default_verification_and_statistics_urls() {
        // We always get the hard-coded defaults regardless of env
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let cfg = load_license_config().unwrap();
        assert_eq!(cfg.verification_url, "https://license.rdatacore.eu/verify");
        assert_eq!(cfg.statistics_url, "https://statistics.rdatacore.eu/submit");
    }
}

// ─── load_cache_config ──────────────────────────────────────────────────────

mod cache_config_tests {
    use super::*;

    #[test]
    fn succeeds_with_explicit_redis_url() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&[("REDIS_URL", Some("redis://testhost:6399"))]);

        let (_, url) = load_cache_config().unwrap();
        assert_eq!(url, "redis://testhost:6399");
    }

    #[test]
    fn cache_explicit_values_override_defaults() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&[
            ("REDIS_URL", Some("redis://localhost:6379")),
            ("CACHE_ENABLED", Some("false")),
            ("CACHE_TTL", Some("120")),
            ("CACHE_MAX_SIZE", Some("500")),
            ("CACHE_ENTITY_DEFINITION_TTL", Some("60")),
            ("CACHE_API_KEY_TTL", Some("900")),
        ]);

        let (cfg, _) = load_cache_config().unwrap();
        assert!(!cfg.enabled);
        assert_eq!(cfg.ttl, 120);
        assert_eq!(cfg.max_size, 500);
        assert_eq!(cfg.entity_definition_ttl, 60);
        assert_eq!(cfg.api_key_ttl, 900);
    }

    #[test]
    fn cache_enabled_true_is_read_correctly() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&[
            ("REDIS_URL", Some("redis://localhost:6379")),
            ("CACHE_ENABLED", Some("true")),
        ]);

        let (cfg, _) = load_cache_config().unwrap();
        assert!(cfg.enabled);
    }

    #[test]
    fn malformed_cache_ttl_falls_back_to_default() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&[
            ("REDIS_URL", Some("redis://localhost:6379")),
            ("CACHE_TTL", Some("not-a-number")),
        ]);

        let (cfg, _) = load_cache_config().unwrap();
        // Malformed values fall back to the hard-coded default (300)
        assert_eq!(cfg.ttl, 300);
    }

    #[test]
    fn malformed_max_size_falls_back_to_default() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&[
            ("REDIS_URL", Some("redis://localhost:6379")),
            ("CACHE_MAX_SIZE", Some("abc")),
        ]);

        let (cfg, _) = load_cache_config().unwrap();
        assert_eq!(cfg.max_size, 10_000);
    }
}

// ─── load_worker_config — value-based tests ─────────────────────────────────

mod worker_config_tests {
    use super::*;

    fn minimal_worker_overrides<'a>() -> Vec<(&'a str, Option<&'a str>)> {
        vec![
            ("JOB_QUEUE_UPDATE_INTERVAL", Some("5")),
            ("WORKER_DATABASE_URL", Some("postgres://localhost/worker")),
            ("REDIS_URL", Some("redis://localhost:6379")),
        ]
    }

    #[test]
    fn fails_when_interval_is_zero() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.push(("JOB_QUEUE_UPDATE_INTERVAL", Some("0")));
        let _env = EnvGuard::new(&overrides);

        let result = load_worker_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must be > 0"), "error: {err}");
    }

    #[test]
    fn fails_when_interval_is_not_a_number() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.push(("JOB_QUEUE_UPDATE_INTERVAL", Some("abc")));
        let _env = EnvGuard::new(&overrides);

        let result = load_worker_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("positive integer"), "error: {err}");
    }

    #[test]
    fn succeeds_with_explicit_worker_vars() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&minimal_worker_overrides());

        let result = load_worker_config();
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
        let cfg = result.unwrap();
        assert_eq!(cfg.job_queue_update_interval_secs, 5);
        assert_eq!(
            cfg.database.connection_string,
            "postgres://localhost/worker"
        );
    }

    #[test]
    fn workflow_config_defaults_when_vars_absent() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.extend_from_slice(&[
            ("WORKFLOW_WORKER_THREADS", None),
            ("WORKFLOW_DEFAULT_TIMEOUT", None),
            ("WORKFLOW_MAX_CONCURRENT", None),
        ]);
        let _env = EnvGuard::new(&overrides);

        let cfg = load_worker_config().unwrap();
        assert_eq!(cfg.workflow.worker_threads, 4);
        assert_eq!(cfg.workflow.default_timeout, 300);
        assert_eq!(cfg.workflow.max_concurrent, 10);
    }

    #[test]
    fn workflow_config_explicit_values() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.extend_from_slice(&[
            ("WORKFLOW_WORKER_THREADS", Some("8")),
            ("WORKFLOW_DEFAULT_TIMEOUT", Some("600")),
            ("WORKFLOW_MAX_CONCURRENT", Some("20")),
        ]);
        let _env = EnvGuard::new(&overrides);

        let cfg = load_worker_config().unwrap();
        assert_eq!(cfg.workflow.worker_threads, 8);
        assert_eq!(cfg.workflow.default_timeout, 600);
        assert_eq!(cfg.workflow.max_concurrent, 20);
    }

    #[test]
    fn large_interval_value_is_accepted() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.push(("JOB_QUEUE_UPDATE_INTERVAL", Some("3600")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_worker_config().unwrap();
        assert_eq!(cfg.job_queue_update_interval_secs, 3600);
    }
}

// ─── outbox config validation ───────────────────────────────────────────────

mod outbox_config_tests {
    use super::*;

    fn minimal_worker_overrides<'a>() -> Vec<(&'a str, Option<&'a str>)> {
        vec![
            ("JOB_QUEUE_UPDATE_INTERVAL", Some("5")),
            ("WORKER_DATABASE_URL", Some("postgres://localhost/worker")),
            ("REDIS_URL", Some("redis://localhost:6379")),
        ]
    }

    #[test]
    fn outbox_flags_read_correctly_when_set() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.extend_from_slice(&[
            ("OUTBOX_ENABLED", Some("true")),
            ("OUTBOX_FETCH_ENABLED", Some("true")),
            ("OUTBOX_PUSH_ENABLED", Some("false")),
        ]);
        let _env = EnvGuard::new(&overrides);

        let cfg = load_worker_config().unwrap();
        assert!(cfg.outbox_enabled);
        assert!(cfg.outbox_fetch_enabled);
        assert!(!cfg.outbox_push_enabled);
    }

    #[test]
    fn outbox_flags_default_to_false() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.extend_from_slice(&[
            ("OUTBOX_ENABLED", Some("false")),
            ("OUTBOX_FETCH_ENABLED", Some("false")),
            ("OUTBOX_PUSH_ENABLED", Some("false")),
        ]);
        let _env = EnvGuard::new(&overrides);

        let cfg = load_worker_config().unwrap();
        assert!(!cfg.outbox_enabled);
        assert!(!cfg.outbox_fetch_enabled);
        assert!(!cfg.outbox_push_enabled);
    }

    #[test]
    fn fails_when_retry_base_delay_is_zero() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.push(("OUTBOX_RETRY_BASE_DELAY_SECS", Some("0")));
        let _env = EnvGuard::new(&overrides);

        let result = load_worker_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("OUTBOX_RETRY_BASE_DELAY_SECS"), "error: {err}");
    }

    #[test]
    fn fails_when_retry_base_delay_is_negative() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.push(("OUTBOX_RETRY_BASE_DELAY_SECS", Some("-5")));
        let _env = EnvGuard::new(&overrides);

        let result = load_worker_config();
        assert!(result.is_err());
    }

    #[test]
    fn fails_when_retry_multiplier_is_one() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.push(("OUTBOX_RETRY_MULTIPLIER", Some("1")));
        let _env = EnvGuard::new(&overrides);

        let result = load_worker_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("OUTBOX_RETRY_MULTIPLIER"), "error: {err}");
    }

    #[test]
    fn fails_when_retry_max_delay_less_than_base_delay() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        // base = 10, max = 5 → invalid
        overrides.extend_from_slice(&[
            ("OUTBOX_RETRY_BASE_DELAY_SECS", Some("10")),
            ("OUTBOX_RETRY_MAX_DELAY_SECS", Some("5")),
        ]);
        let _env = EnvGuard::new(&overrides);

        let result = load_worker_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("OUTBOX_RETRY_MAX_DELAY_SECS"), "error: {err}");
    }

    #[test]
    fn fails_when_retry_base_delay_is_non_numeric() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.push(("OUTBOX_RETRY_BASE_DELAY_SECS", Some("bad")));
        let _env = EnvGuard::new(&overrides);

        let result = load_worker_config();
        assert!(result.is_err());
    }

    #[test]
    fn fails_when_stale_lease_secs_is_zero() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.push(("OUTBOX_STALE_LEASE_SECS", Some("0")));
        let _env = EnvGuard::new(&overrides);

        let result = load_worker_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("OUTBOX_STALE_LEASE_SECS"), "error: {err}");
    }

    #[test]
    fn outbox_retry_explicit_values_accepted() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_worker_overrides();
        overrides.extend_from_slice(&[
            ("OUTBOX_RETRY_BASE_DELAY_SECS", Some("2")),
            ("OUTBOX_RETRY_MULTIPLIER", Some("3")),
            ("OUTBOX_RETRY_MAX_DELAY_SECS", Some("600")),
            ("OUTBOX_STALE_LEASE_SECS", Some("120")),
        ]);
        let _env = EnvGuard::new(&overrides);

        let cfg = load_worker_config().unwrap();
        assert_eq!(cfg.outbox_retry_base_delay_secs, 2);
        assert_eq!(cfg.outbox_retry_multiplier, 3);
        assert_eq!(cfg.outbox_retry_max_delay_secs, 600);
        assert_eq!(cfg.outbox_stale_lease_secs, 120);
    }
}

// ─── load_app_config ────────────────────────────────────────────────────────

mod app_config_tests {
    use super::*;
    use crate::config::load_app_config;

    fn minimal_app_overrides<'a>() -> Vec<(&'a str, Option<&'a str>)> {
        vec![
            ("DATABASE_URL", Some("postgres://localhost/app")),
            ("JWT_SECRET", Some("test-jwt-secret")),
            ("REDIS_URL", Some("redis://localhost:6379")),
        ]
    }

    #[test]
    fn succeeds_and_reads_database_url() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&minimal_app_overrides());

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.database.connection_string, "postgres://localhost/app");
        assert_eq!(cfg.api.jwt_secret, "test-jwt-secret");
    }

    #[test]
    fn app_env_production_is_read() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("APP_ENV", Some("production")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.environment, "production");
        assert!(cfg.is_production());
    }

    #[test]
    fn app_env_staging_is_not_production() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("APP_ENV", Some("staging")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.environment, "staging");
        assert!(!cfg.is_production());
    }

    #[test]
    fn cors_origins_split_on_comma() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push((
            "CORS_ORIGINS",
            Some("https://a.com,https://b.com, https://c.com"),
        ));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(
            cfg.api.cors_origins,
            vec![
                "https://a.com".to_string(),
                "https://b.com".to_string(),
                "https://c.com".to_string(),
            ]
        );
    }

    #[test]
    fn cors_origins_single_entry() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("CORS_ORIGINS", Some("https://only.com")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.api.cors_origins, vec!["https://only.com".to_string()]);
    }

    #[test]
    fn api_host_and_port_explicit_values() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.extend_from_slice(&[("API_HOST", Some("127.0.0.1")), ("API_PORT", Some("9999"))]);
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.api.host, "127.0.0.1");
        assert_eq!(cfg.api.port, 9999);
    }

    #[test]
    fn jwt_expiration_explicit_value() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("JWT_EXPIRATION", Some("3600")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.api.jwt_expiration, 3600);
    }

    #[test]
    fn database_max_connections_explicit_value() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("DATABASE_MAX_CONNECTIONS", Some("25")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.database.max_connections, 25);
    }

    #[test]
    fn password_reset_throttle_explicit_value() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("PASSWORD_RESET_THROTTLE_SECONDS", Some("120")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.password_reset_throttle_seconds, 120);
    }

    #[test]
    fn frontend_base_url_empty_string_becomes_none() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("FRONTEND_BASE_URL", Some("")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert!(cfg.frontend_base_url.is_none());
    }

    #[test]
    fn frontend_base_url_set_correctly() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("FRONTEND_BASE_URL", Some("https://app.example.com")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(
            cfg.frontend_base_url.as_deref(),
            Some("https://app.example.com")
        );
    }

    #[test]
    fn queue_keys_explicit_values() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.extend_from_slice(&[
            ("QUEUE_FETCH_KEY", Some("custom:fetch")),
            ("QUEUE_PROCESS_KEY", Some("custom:process")),
            ("QUEUE_EMAIL_KEY", Some("custom:email")),
        ]);
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.queue.fetch_key, "custom:fetch");
        assert_eq!(cfg.queue.process_key, "custom:process");
        assert_eq!(cfg.queue.email_key, "custom:email");
    }

    #[test]
    fn log_level_explicit_value() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.extend_from_slice(&[
            ("LOG_LEVEL", Some("debug")),
            ("LOG_FILE", Some("/var/log/app.log")),
        ]);
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.log.level, "debug");
        assert_eq!(cfg.log.file.as_deref(), Some("/var/log/app.log"));
    }

    #[test]
    fn api_use_tls_can_be_enabled() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("API_USE_TLS", Some("true")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert!(cfg.api.use_tls);
    }

    #[test]
    fn api_enable_docs_can_be_disabled() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("API_ENABLE_DOCS", Some("false")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert!(!cfg.api.enable_docs);
    }

    #[test]
    fn check_default_admin_password_can_be_disabled() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("CHECK_DEFAULT_ADMIN_PASSWORD", Some("false")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert!(!cfg.api.check_default_admin_password);
    }

    #[test]
    fn database_connection_timeout_explicit_value() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_app_overrides();
        overrides.push(("DATABASE_CONNECTION_TIMEOUT", Some("60")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_app_config().unwrap();
        assert_eq!(cfg.database.connection_timeout, 60);
    }
}

// ─── load_maintenance_config ────────────────────────────────────────────────

mod maintenance_config_tests {
    use super::*;
    use crate::config::load_maintenance_config;

    fn minimal_maintenance_overrides<'a>() -> Vec<(&'a str, Option<&'a str>)> {
        vec![
            (
                "MAINTENANCE_DATABASE_URL",
                Some("postgres://localhost/maint"),
            ),
            ("REDIS_URL", Some("redis://localhost:6379")),
            ("API_HOST", Some("0.0.0.0")),
            ("API_PORT", Some("8888")),
            ("JWT_SECRET", Some("test-secret")),
            ("CORS_ORIGINS", Some("https://app.example.com")),
            ("VERSION_PURGER_CRON", Some("0 0 * * * *")),
            ("REFRESH_TOKEN_CLEANUP_CRON", Some("0 30 * * * *")),
            ("WORKFLOW_RUN_LOGS_PURGER_CRON", Some("0 0 1 * * *")),
            ("SYSTEM_LOGS_PURGER_CRON", Some("0 0 2 * * *")),
            ("OUTBOX_ENABLED", Some("false")),
        ]
    }

    #[test]
    fn succeeds_with_minimal_maintenance_vars() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::new(&minimal_maintenance_overrides());

        let result = load_maintenance_config();
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
        let cfg = result.unwrap();
        assert_eq!(cfg.database.connection_string, "postgres://localhost/maint");
        assert!(!cfg.outbox_enabled);
        assert!(cfg.outbox_purger_cron.is_none());
        assert!(cfg.outbox_retention_days.is_none());
    }

    #[test]
    fn fails_when_cron_expression_is_invalid() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_maintenance_overrides();
        overrides.push(("VERSION_PURGER_CRON", Some("not-a-cron")));
        let _env = EnvGuard::new(&overrides);

        let result = load_maintenance_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("VERSION_PURGER_CRON") || err.contains("Invalid"),
            "error: {err}"
        );
    }

    #[test]
    fn system_logs_retention_defaults_to_90() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_maintenance_overrides();
        overrides.push(("SYSTEM_LOGS_RETENTION_DAYS", None));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_maintenance_config().unwrap();
        assert_eq!(cfg.system_logs_retention_days, 90);
    }

    #[test]
    fn system_logs_retention_explicit_value() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_maintenance_overrides();
        overrides.push(("SYSTEM_LOGS_RETENTION_DAYS", Some("30")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_maintenance_config().unwrap();
        assert_eq!(cfg.system_logs_retention_days, 30);
    }

    #[test]
    fn outbox_purger_cron_required_when_outbox_enabled() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_maintenance_overrides();
        overrides.extend_from_slice(&[
            ("OUTBOX_ENABLED", Some("true")),
            ("OUTBOX_PURGER_CRON", None),
        ]);
        let _env = EnvGuard::new(&overrides);

        let result = load_maintenance_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("OUTBOX_PURGER_CRON"), "error: {err}");
    }

    #[test]
    fn outbox_purger_cron_loaded_when_outbox_enabled() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_maintenance_overrides();
        overrides.extend_from_slice(&[
            ("OUTBOX_ENABLED", Some("true")),
            ("OUTBOX_PURGER_CRON", Some("0 0 3 * * *")),
        ]);
        let _env = EnvGuard::new(&overrides);

        let result = load_maintenance_config();
        assert!(result.is_ok(), "error: {:?}", result.err());
        let cfg = result.unwrap();
        assert!(cfg.outbox_purger_cron.is_some());
        assert!(cfg.outbox_retention_days.is_some());
    }

    #[test]
    fn fails_when_api_port_not_a_number() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_maintenance_overrides();
        overrides.push(("API_PORT", Some("not-a-port")));
        let _env = EnvGuard::new(&overrides);

        let result = load_maintenance_config();
        assert!(result.is_err());
    }

    #[test]
    fn maintenance_api_cors_origins_split_correctly() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_maintenance_overrides();
        overrides.push(("CORS_ORIGINS", Some("https://x.com, https://y.com")));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_maintenance_config().unwrap();
        assert_eq!(
            cfg.api.cors_origins,
            vec!["https://x.com".to_string(), "https://y.com".to_string()]
        );
    }

    #[test]
    fn outbox_retention_days_explicit_value() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_maintenance_overrides();
        overrides.extend_from_slice(&[
            ("OUTBOX_ENABLED", Some("true")),
            ("OUTBOX_PURGER_CRON", Some("0 0 3 * * *")),
            ("OUTBOX_RETENTION_DAYS", Some("60")),
        ]);
        let _env = EnvGuard::new(&overrides);

        let cfg = load_maintenance_config().unwrap();
        assert_eq!(cfg.outbox_retention_days, Some(60_u32));
    }

    #[test]
    fn maintenance_database_connection_timeout_default() {
        let _mutex = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut overrides = minimal_maintenance_overrides();
        overrides.push(("MAINTENANCE_DATABASE_CONNECTION_TIMEOUT", None));
        let _env = EnvGuard::new(&overrides);

        let cfg = load_maintenance_config().unwrap();
        assert_eq!(cfg.database.connection_timeout, 30);
    }
}
