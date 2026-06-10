#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;
use std::time::Duration;

use log::{error, info};
use sqlx::postgres::PgListener;
use tokio::sync::Notify;

use r_data_core_core::outbox::WORKFLOW_OUTBOX_NOTIFY_CHANNEL;

const OUTBOX_LISTENER_RECONNECT_INITIAL_DELAY_SECS: u64 = 1;
const OUTBOX_LISTENER_RECONNECT_MAX_DELAY_SECS: u64 = 30;

pub(super) async fn run_workflow_outbox_notification_listener(
    outbox_db_url: String,
    outbox_notify: Arc<Notify>,
) {
    let mut reconnect_delay = Duration::from_secs(OUTBOX_LISTENER_RECONNECT_INITIAL_DELAY_SECS);

    loop {
        match PgListener::connect(&outbox_db_url).await {
            Ok(mut listener) => {
                if let Err(e) = listener.listen(WORKFLOW_OUTBOX_NOTIFY_CHANNEL).await {
                    error!(
                        "Failed to listen for workflow outbox notifications on '{WORKFLOW_OUTBOX_NOTIFY_CHANNEL}': {e}"
                    );
                } else {
                    info!(
                        "Workflow outbox notification listener attached to '{WORKFLOW_OUTBOX_NOTIFY_CHANNEL}'"
                    );

                    reconnect_delay =
                        Duration::from_secs(OUTBOX_LISTENER_RECONNECT_INITIAL_DELAY_SECS);

                    loop {
                        match listener.recv().await {
                            Ok(_notification) => {
                                outbox_notify.notify_one();
                            }
                            Err(e) => {
                                error!(
                                    "Workflow outbox notification listener failed: {e}; reconnecting in {}s",
                                    reconnect_delay.as_secs()
                                );
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    "Failed to initialize workflow outbox notification listener: {e}; reconnecting in {}s",
                    reconnect_delay.as_secs()
                );
            }
        }

        tokio::time::sleep(reconnect_delay).await;
        reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(
            OUTBOX_LISTENER_RECONNECT_MAX_DELAY_SECS,
        ));
    }
}
