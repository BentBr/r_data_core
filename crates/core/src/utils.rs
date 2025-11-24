#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use chrono::{DateTime, Utc};
use cron::Schedule;
use std::str::FromStr;

/// Validate a cron expression
///
/// # Arguments
/// * `expr` - The cron expression to validate
///
/// # Errors
/// Returns an error if the cron expression is invalid
pub fn validate_cron(expr: &str) -> Result<(), String> {
    Schedule::from_str(expr).map_err(|e| format!("Invalid cron expression: {e}"))?;
    Ok(())
}

/// Preview the next N occurrences of a cron schedule
///
/// # Arguments
/// * `expr` - The cron expression
/// * `count` - Number of occurrences to preview
///
/// # Returns
/// Vector of RFC3339 formatted datetime strings
///
/// # Errors
/// Returns an error if the cron expression is invalid
pub fn preview_next(expr: &str, count: usize) -> Result<Vec<String>, String> {
    let schedule = Schedule::from_str(expr).map_err(|e| format!("Invalid cron expression: {e}"))?;
    let times: Vec<String> = schedule
        .upcoming(Utc)
        .take(count)
        .map(|dt: DateTime<Utc>| dt.to_rfc3339())
        .collect();
    Ok(times)
}

