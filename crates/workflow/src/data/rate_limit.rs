#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};

/// Optional per-workflow request limit, stored inside `Workflow.config` under
/// the `rate_limit` key.
///
/// Deliberately per workflow and opt-in: a global limit on the public API would
/// let one workflow's traffic throttle every other workflow's consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRateLimit {
    /// Whether the limit is applied at all.
    pub enabled: bool,
    /// Requests permitted per window.
    pub max_requests: u32,
    /// Window length in minutes, as configured by the admin.
    pub window_minutes: u32,
}

impl WorkflowRateLimit {
    /// Read the limit out of a workflow config.
    ///
    /// Returns `None` when absent, disabled, malformed, or nonsensical. A bad
    /// config must never throttle: failing closed here would take a working
    /// workflow offline because of an unrelated bad edit.
    #[must_use]
    pub fn from_config(config: &serde_json::Value) -> Option<Self> {
        let limit: Self = serde_json::from_value(config.get("rate_limit")?.clone()).ok()?;

        if !limit.enabled || limit.max_requests == 0 || limit.window_minutes == 0 {
            return None;
        }

        Some(limit)
    }

    /// Window length in seconds, for the counter's TTL.
    #[must_use]
    pub const fn window_secs(&self) -> u64 {
        self.window_minutes as u64 * 60
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::WorkflowRateLimit;
    use serde_json::json;

    #[test]
    fn absent_config_means_no_limit() {
        assert!(WorkflowRateLimit::from_config(&json!({})).is_none());
        assert!(WorkflowRateLimit::from_config(&json!({ "other": 1 })).is_none());
    }

    /// Disabled must behave exactly like absent, so toggling the checkbox off
    /// in the UI actually turns the limit off rather than leaving a stale one.
    #[test]
    fn disabled_config_means_no_limit() {
        let cfg =
            json!({ "rate_limit": { "enabled": false, "max_requests": 10, "window_minutes": 60 } });
        assert!(WorkflowRateLimit::from_config(&cfg).is_none());
    }

    #[test]
    fn enabled_config_is_parsed() {
        let cfg =
            json!({ "rate_limit": { "enabled": true, "max_requests": 10, "window_minutes": 60 } });
        let limit = WorkflowRateLimit::from_config(&cfg).unwrap();

        assert_eq!(limit.max_requests, 10);
        assert_eq!(limit.window_minutes, 60);
        assert_eq!(limit.window_secs(), 3600);
    }

    /// A malformed block must not throttle anything — failing closed here would
    /// take a working workflow offline because of a bad edit elsewhere.
    #[test]
    fn malformed_config_means_no_limit() {
        for cfg in [
            json!({ "rate_limit": "yes" }),
            json!({ "rate_limit": { "enabled": true } }),
            json!({ "rate_limit": { "enabled": true, "max_requests": "ten", "window_minutes": 60 } }),
        ] {
            assert!(
                WorkflowRateLimit::from_config(&cfg).is_none(),
                "malformed config must not produce a limit: {cfg}"
            );
        }
    }

    /// Zero would either block everything or divide by zero downstream.
    #[test]
    fn zero_values_are_rejected() {
        let zero_max =
            json!({ "rate_limit": { "enabled": true, "max_requests": 0, "window_minutes": 60 } });
        let zero_window =
            json!({ "rate_limit": { "enabled": true, "max_requests": 10, "window_minutes": 0 } });

        assert!(WorkflowRateLimit::from_config(&zero_max).is_none());
        assert!(WorkflowRateLimit::from_config(&zero_window).is_none());
    }

    #[test]
    fn window_secs_converts_minutes() {
        let cfg =
            json!({ "rate_limit": { "enabled": true, "max_requests": 5, "window_minutes": 1 } });
        assert_eq!(
            WorkflowRateLimit::from_config(&cfg).unwrap().window_secs(),
            60
        );
    }
}
