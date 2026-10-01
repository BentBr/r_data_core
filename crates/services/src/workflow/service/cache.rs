#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use uuid::Uuid;

use super::WorkflowService;

/// How long a cached workflow may be served, in seconds.
///
/// Deliberately short, and deliberately NOT the default cache TTL (3600s).
/// `CacheManager` keeps a per-process in-memory tier that an invalidation in
/// another process never clears — `SettingsService` documents the same hazard
/// and solves it the same way. The API and the workflow worker are separate
/// processes, and a workflow's config carries its pre-shared key, so an
/// unbounded entry would let a rotated or revoked key keep working elsewhere
/// for up to an hour. This TTL is the correctness bound; the invalidation
/// below is only a latency optimisation for the writing process.
pub const WORKFLOW_CACHE_TTL_SECS: u64 = 10;

// Enforced at compile time rather than in a test: the value is a constant, so a
// runtime assertion could never fail anyway, and this way raising it past the
// bound stops the build instead of a test run.
const _: () = assert!(
    WORKFLOW_CACHE_TTL_SECS > 0,
    "zero would disable caching entirely"
);
const _: () = assert!(
    WORKFLOW_CACHE_TTL_SECS <= 60,
    "a workflow config holds a pre-shared key; this is too long to serve a stale one"
);

impl WorkflowService {
    /// Cache key for a workflow by UUID.
    #[must_use]
    pub fn cache_key_by_uuid(uuid: &Uuid) -> String {
        format!("workflow:by_uuid:{uuid}")
    }

    /// Drop the cached copy of a workflow.
    ///
    /// Called after every mutation so an edited rate limit or rotated key takes
    /// effect on the next request in this process, rather than after the TTL.
    pub(crate) async fn invalidate_workflow_cache(&self, uuid: &Uuid) {
        let Some(cache) = &self.cache_manager else {
            return;
        };
        if let Err(e) = cache.delete(&Self::cache_key_by_uuid(uuid)).await {
            log::warn!("Failed to invalidate workflow cache for {uuid}: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WorkflowService;
    use uuid::Uuid;

    #[test]
    fn cache_key_is_namespaced_by_uuid() {
        let a = Uuid::now_v7();
        let b = Uuid::now_v7();

        assert!(WorkflowService::cache_key_by_uuid(&a).starts_with("workflow:by_uuid:"));
        assert_ne!(
            WorkflowService::cache_key_by_uuid(&a),
            WorkflowService::cache_key_by_uuid(&b),
            "two workflows must never share a cache entry"
        );
    }
}
