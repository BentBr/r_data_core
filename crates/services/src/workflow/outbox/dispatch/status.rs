use r_data_core_core::outbox::{OutboxMessage, OutboxStatus};

/// Convert a fetched outbox record into a simplified state view.
#[must_use]
pub const fn outbox_status(record: &OutboxMessage) -> OutboxStatus {
    record.status
}
