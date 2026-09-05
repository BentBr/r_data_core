use super::*;

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
