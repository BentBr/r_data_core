#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;
use std::time::Duration;

use log::{error, info};
use time::OffsetDateTime;
use tokio::sync::Notify;
use uuid::Uuid;

use r_data_core_services::workflow::outbox::DispatchWorkflowOutboxBatchUseCase;

use crate::runtime::WorkerRuntime;

use super::listener::run_workflow_outbox_notification_listener;

pub fn spawn_outbox_recovery_loop(runtime: &WorkerRuntime) {
    let Some(outbox_repo_for_outbox) = runtime.outbox_repo.clone() else {
        return;
    };

    let queue_for_outbox = runtime.queue.clone();
    let outbox_notify = Arc::new(Notify::new());

    {
        let outbox_db_url = runtime.outbox_db_url.clone();
        let outbox_notify = outbox_notify.clone();
        tokio::spawn(run_workflow_outbox_notification_listener(
            outbox_db_url,
            outbox_notify,
        ));
    }

    let outbox_retry_policy = runtime.outbox_retry_policy;
    let outbox_stale_lease_secs = runtime.outbox_stale_lease_secs;
    let poll_interval =
        Duration::from_secs(std::cmp::max(5, runtime.job_queue_update_interval_secs));

    tokio::spawn(async move {
        const OUTBOX_BATCH_SIZE: i64 = 50;
        let worker_id = format!("workflow-outbox-{}", Uuid::now_v7());
        let mut sleep_until = std::time::Instant::now();

        loop {
            tokio::select! {
                () = tokio::time::sleep_until(tokio::time::Instant::from_std(sleep_until)) => {}
                () = outbox_notify.notified() => {}
            }

            let mut dispatched_total = 0usize;
            loop {
                let dispatch_use_case = DispatchWorkflowOutboxBatchUseCase::new(
                    queue_for_outbox.as_ref(),
                    outbox_repo_for_outbox.as_ref(),
                    worker_id.as_str(),
                    OUTBOX_BATCH_SIZE,
                    outbox_stale_lease_secs,
                    outbox_retry_policy.as_ref(),
                );
                match dispatch_use_case.run_once().await {
                    Ok(dispatched) => {
                        if dispatched == 0 {
                            break;
                        }
                        dispatched_total = dispatched_total.saturating_add(dispatched);
                    }
                    Err(e) => {
                        error!("Workflow outbox dispatcher failed: {e}");
                        break;
                    }
                }
            }

            if dispatched_total > 0 {
                info!(
                    "Dispatched {dispatched_total} workflow outbox message(s) via worker outbox loop"
                );
            }

            let next_available_at = match outbox_repo_for_outbox.next_available_at().await {
                Ok(value) => value,
                Err(e) => {
                    error!("Failed to query next workflow outbox availability: {e}");
                    None
                }
            };

            let now = std::time::Instant::now();
            let fallback = now.checked_add(poll_interval).unwrap_or(now);
            sleep_until = next_available_at
                .and_then(|value| {
                    let now_utc = OffsetDateTime::now_utc();
                    if value <= now_utc {
                        Some(now)
                    } else {
                        let delta = value - now_utc;
                        let secs = u64::try_from(delta.whole_seconds()).unwrap_or(0);
                        now.checked_add(Duration::from_secs(secs))
                    }
                })
                .unwrap_or(fallback);
        }
    });
}
