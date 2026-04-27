#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

mod dispatch;
mod payload;
mod policy;
mod support;
mod use_cases;

pub use dispatch::{enqueue_workflow_push_outbox, outbox_status, WorkflowOutboxDispatcher};
pub(crate) use payload::validate_workflow_push_outbox_size;
pub use payload::WorkflowPushOutboxPayload;
pub use policy::{workflow_outbox_retry_at, workflow_outbox_retry_delay_secs, OutboxRetryPolicy};
pub use use_cases::{DispatchWorkflowOutboxBatchUseCase, EnqueueWorkflowFetchUseCase};

pub const WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES: usize = 256 * 1024;
pub const WORKFLOW_OUTBOX_STALE_LEASE_SECS: i64 = 300;
pub const WORKFLOW_OUTBOX_MAX_ATTEMPTS: i32 = 10;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_uses_base_for_first_attempt() {
        let policy = OutboxRetryPolicy::new(3, 2, 300);
        assert_eq!(workflow_outbox_retry_delay_secs(1, &policy), 3);
    }

    #[test]
    fn retry_delay_scales_exponentially_until_cap() {
        let policy = OutboxRetryPolicy::new(2, 3, 20);
        assert_eq!(workflow_outbox_retry_delay_secs(1, &policy), 2);
        assert_eq!(workflow_outbox_retry_delay_secs(2, &policy), 6);
        assert_eq!(workflow_outbox_retry_delay_secs(3, &policy), 18);
        assert_eq!(workflow_outbox_retry_delay_secs(4, &policy), 20);
        assert_eq!(workflow_outbox_retry_delay_secs(10, &policy), 20);
    }

    #[test]
    fn retry_delay_clamps_invalid_policy_values() {
        let policy = OutboxRetryPolicy::new(0, 1, 0);
        assert_eq!(workflow_outbox_retry_delay_secs(1, &policy), 1);
        assert_eq!(workflow_outbox_retry_delay_secs(5, &policy), 1);
    }

    #[test]
    fn push_payload_size_validation_accepts_small_payload() {
        assert!(validate_workflow_push_outbox_size(&[0u8; 1024]).is_ok());
    }

    #[test]
    fn push_payload_size_validation_rejects_large_payload() {
        let payload = vec![0u8; WORKFLOW_PUSH_OUTBOX_MAX_DATA_BYTES + 1];
        let result = validate_workflow_push_outbox_size(&payload);
        assert!(
            matches!(result, Err(r_data_core_core::error::Error::Validation(message)) if message.contains("maximum size"))
        );
    }
}
