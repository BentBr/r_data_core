use super::*;

mod outbox_config_tests {
    use super::*;

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
