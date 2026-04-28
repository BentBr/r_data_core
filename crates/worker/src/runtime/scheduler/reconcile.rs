#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::collections::HashMap;

use uuid::Uuid;

use r_data_core_persistence::WorkflowRepository;

use crate::runtime::WorkerBootstrap;

use super::jobs::{schedule_workflow_job, ScheduleWorkflowJobConfig};

pub(super) fn spawn_reconcile_task(bootstrap: &WorkerBootstrap) {
    let scheduler_clone = bootstrap.scheduler.clone();
    let repo_clone = WorkflowRepository::new(bootstrap.runtime.pool.clone());
    let pool_clone = bootstrap.runtime.pool.clone();
    let cache_manager_clone = bootstrap.runtime.cache_manager.clone();
    let outbox_fetch_enabled_default = bootstrap.runtime.outbox_fetch_enabled_default;
    let outbox_push_enabled_default = bootstrap.runtime.outbox_push_enabled_default;
    let scheduled_map = bootstrap.scheduled_workflows.clone();
    let queue_for_reconcile = bootstrap.runtime.queue.clone();
    let outbox_repo_for_reconcile = bootstrap.runtime.outbox_repo.clone();
    let outbox_retry_policy = bootstrap.runtime.outbox_retry_policy;
    let interval_secs = bootstrap.runtime.job_queue_update_interval_secs;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            if let Ok(db_workflows) = repo_clone.list_scheduled_consumers().await {
                let mut map = scheduled_map.lock().await;
                let current_set: HashMap<Uuid, String> = db_workflows
                    .iter()
                    .map(|(wf_id, cron)| (*wf_id, cron.clone()))
                    .collect();
                let existing_set: HashMap<Uuid, String> = map
                    .iter()
                    .map(|(wf_id, (_job_id, cron))| (*wf_id, cron.clone()))
                    .collect();
                let (wf_to_remove, wf_to_add) =
                    r_data_core_services::compute_reconcile_actions(&existing_set, &current_set);
                for wf_id in wf_to_remove {
                    if let Some((job_id, _)) = map.get(&wf_id) {
                        let _ = scheduler_clone.remove(job_id).await;
                    }
                    map.remove(&wf_id);
                }
                for (wf_id, cron) in wf_to_add {
                    let job_cfg = ScheduleWorkflowJobConfig {
                        pool: pool_clone.clone(),
                        cache_manager: cache_manager_clone.clone(),
                        outbox_fetch_enabled_default,
                        outbox_push_enabled_default,
                        queue: queue_for_reconcile.clone(),
                        outbox_repo: outbox_repo_for_reconcile.clone(),
                        outbox_retry_policy,
                    };
                    if let Ok(job_id) =
                        schedule_workflow_job(scheduler_clone.clone(), wf_id, cron.clone(), job_cfg)
                            .await
                    {
                        map.insert(wf_id, (job_id, cron));
                    }
                }
            }
        }
    });
}
