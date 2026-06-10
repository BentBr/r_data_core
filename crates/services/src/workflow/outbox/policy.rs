#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use time::OffsetDateTime;

/// Retry policy for workflow outbox entries.
#[derive(Debug, Clone, Copy)]
pub struct OutboxRetryPolicy {
    pub base_delay_secs: i64,
    pub multiplier: u64,
    pub max_delay_secs: i64,
}

impl Default for OutboxRetryPolicy {
    fn default() -> Self {
        Self {
            base_delay_secs: 1,
            multiplier: 2,
            max_delay_secs: 300,
        }
    }
}

impl OutboxRetryPolicy {
    #[must_use]
    pub const fn new(base_delay_secs: i64, multiplier: u64, max_delay_secs: i64) -> Self {
        Self {
            base_delay_secs,
            multiplier,
            max_delay_secs,
        }
    }
}

/// Compute the delay in seconds for the next workflow outbox retry.
#[must_use]
pub fn workflow_outbox_retry_delay_secs(attempt_count: i32, policy: &OutboxRetryPolicy) -> i64 {
    let exponent = u32::try_from(attempt_count.saturating_sub(1).clamp(0, 31)).unwrap_or(0);
    let multiplier = i128::from(policy.multiplier.max(1));
    let cap = i128::from(policy.max_delay_secs.max(1));
    let mut delay_secs = i128::from(policy.base_delay_secs.max(1));

    for _ in 0..exponent {
        delay_secs = delay_secs.saturating_mul(multiplier).min(cap);
        if delay_secs >= cap {
            return i64::try_from(cap).unwrap_or(i64::MAX);
        }
    }

    i64::try_from(delay_secs.min(cap)).unwrap_or(i64::MAX)
}

/// Compute the next retry timestamp using a capped exponential backoff.
#[must_use]
pub fn workflow_outbox_retry_at(attempt_count: i32, policy: &OutboxRetryPolicy) -> OffsetDateTime {
    OffsetDateTime::now_utc()
        + time::Duration::seconds(workflow_outbox_retry_delay_secs(attempt_count, policy))
}
