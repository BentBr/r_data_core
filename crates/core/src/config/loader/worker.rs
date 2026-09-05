#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use dotenvy::dotenv;
use std::env;

use crate::config::WorkerConfig;
use crate::error::Result;

use super::shared::{
    get_cache_config, get_license_config, get_mail_config, get_queue_config, load_outbox_config,
    load_worker_database_config, load_workflow_config,
};

/// Load worker configuration from environment variables
///
/// # Errors
/// Returns an error if required environment variables are missing or invalid
pub fn load_worker_config() -> Result<WorkerConfig> {
    // Ensure .env is loaded for binaries that only use WorkerConfig
    dotenv().ok();

    let interval_str = env::var("JOB_QUEUE_UPDATE_INTERVAL").map_err(|_| {
        crate::error::Error::Config("JOB_QUEUE_UPDATE_INTERVAL not set".to_string())
    })?;
    let job_queue_update_interval_secs = interval_str.parse::<u64>().map_err(|_| {
        crate::error::Error::Config(
            "JOB_QUEUE_UPDATE_INTERVAL must be a positive integer (seconds)".to_string(),
        )
    })?;
    if job_queue_update_interval_secs == 0 {
        return Err(crate::error::Error::Config(
            "JOB_QUEUE_UPDATE_INTERVAL must be > 0 seconds".to_string(),
        ));
    }
    let outbox = load_outbox_config(true)?;
    let database = load_worker_database_config()?;
    let workflow = load_workflow_config();

    let queue = get_queue_config()?;
    let cache = get_cache_config();
    let license = get_license_config();
    let mail = get_mail_config();

    Ok(WorkerConfig {
        job_queue_update_interval_secs,
        outbox_enabled: outbox.enabled,
        outbox_fetch_enabled: outbox.fetch_enabled,
        outbox_push_enabled: outbox.push_enabled,
        outbox_stale_lease_secs: outbox.stale_lease_secs,
        outbox_retry_base_delay_secs: outbox.retry_base_delay_secs,
        outbox_retry_multiplier: outbox.retry_multiplier,
        outbox_retry_max_delay_secs: outbox.retry_max_delay_secs,
        database,
        workflow,
        queue,
        cache,
        license,
        mail,
    })
}
