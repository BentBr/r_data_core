use r_data_core::utils::cron::{preview_next, validate_cron};

#[test]
fn test_validate_cron_success() {
    assert!(validate_cron("*/5 * * * *").is_ok());
}

#[test]
fn test_validate_cron_failure() {
    assert!(validate_cron("not-a-cron").is_err());
}

#[test]
fn test_preview_next_returns_items() {
    let items = preview_next("*/5 * * * *", 3).unwrap();
    assert_eq!(items.len(), 3);
    // Ensure ISO strings
    assert!(items[0].contains('T'));
}
