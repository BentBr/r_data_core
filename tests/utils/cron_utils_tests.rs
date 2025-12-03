#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use r_data_core_core::utils::{preview_next, validate_cron};

#[test]
fn test_validate_cron_success() {
    // Cron 0.12 requires 6 fields (with seconds)
    assert!(validate_cron("0 */5 * * * *").is_ok());
}

#[test]
fn test_validate_cron_failure() {
    assert!(validate_cron("not-a-cron").is_err());
}

#[test]
fn test_preview_next_returns_items() {
    // Cron 0.12 requires 6 fields (with seconds)
    let items = preview_next("0 */5 * * * *", 3).unwrap();
    assert_eq!(items.len(), 3);
    // Ensure ISO strings
    assert!(items[0].contains('T'));
}
