#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::time::Duration;

use log::{error, info};

use super::handler::handle_email_job;
use super::state::EmailConsumerState;
use super::EmailRuntime;

pub fn spawn_email_consumer_loop(runtime: &EmailRuntime) {
    let state = EmailConsumerState::from_runtime(runtime);

    tokio::spawn(async move {
        info!(
            "Email consumer loop started, waiting for email jobs from queue '{}'...",
            state.queue_email_key
        );
        consume_email_loop(state).await;
    });
}

async fn consume_email_loop(state: EmailConsumerState) {
    let mut retry_backoff_ms: u64 = 250;

    loop {
        match state.queue.blocking_pop_email().await {
            Ok(job) => {
                retry_backoff_ms = 250;
                info!("Popped email job: to={:?}, source={}", job.to, job.source);
                handle_email_job(&state, job).await;
            }
            Err(e) => {
                error!("Email queue pop failed: {e}");
                tokio::time::sleep(Duration::from_millis(retry_backoff_ms)).await;
                retry_backoff_ms = (retry_backoff_ms * 2).min(30_000);
            }
        }
    }
}
