use chrono::{DateTime, Utc};
use cron::Schedule;
use std::str::FromStr;

pub fn validate_cron(expr: &str) -> anyhow::Result<()> {
    let _ = Schedule::from_str(expr)?;
    Ok(())
}

pub fn preview_next(expr: &str, count: usize) -> anyhow::Result<Vec<String>> {
    let schedule = Schedule::from_str(expr)?;
    let times: Vec<String> = schedule
        .upcoming(Utc)
        .take(count)
        .map(|dt: DateTime<Utc>| dt.to_rfc3339())
        .collect();
    Ok(times)
}
