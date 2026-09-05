use super::*;

mod worker_config_tests {
    use super::*;

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
