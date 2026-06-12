use super::*;
use crate::config::load_app_config;

fn minimal_app_overrides<'a>() -> Vec<(&'a str, Option<&'a str>)> {
    vec![
        ("DATABASE_URL", Some("postgres://localhost/app")),
        ("JWT_SECRET", Some("test-jwt-secret")),
        ("REDIS_URL", Some("redis://localhost:6379")),
    ]
}

mod app_config_tests {
    use super::*;

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
