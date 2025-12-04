#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
pub mod api_key_cache_tests;
pub mod entity_definition_cache_tests;

/// Helper function to create a `CacheManager` with in-memory cache for tests
#[must_use] 
pub fn create_test_cache_manager() -> std::sync::Arc<r_data_core_core::cache::CacheManager> {
    use r_data_core_core::cache::CacheManager;
    use r_data_core_core::config::CacheConfig;
    let config = CacheConfig {
        entity_definition_ttl: 0, // No expiration
        api_key_ttl: 600,         // 10 minutes for tests
        enabled: true,
        ttl: 3600, // 1-hour default
        max_size: 10000,
    };

    std::sync::Arc::new(CacheManager::new(config))
}
