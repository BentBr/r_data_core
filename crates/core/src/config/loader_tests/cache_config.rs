use super::*;

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
