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

mod maintenance_config_tests {
    use super::*;

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
