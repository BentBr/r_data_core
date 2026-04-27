use r_data_core_core::outbox::OutboxStatus;
use r_data_core_persistence::OutboxMessageRecord;

/// Convert a fetched outbox record into a simplified state view.
#[must_use]
pub fn outbox_status(record: &OutboxMessageRecord) -> OutboxStatus {
    record
        .status
        .parse::<OutboxStatus>()
        .unwrap_or(OutboxStatus::Pending)
}
