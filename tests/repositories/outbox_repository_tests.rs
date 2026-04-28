#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use httpmock::{Method::POST, MockServer};
use r_data_core_persistence::WorkflowRepositoryTrait;
use r_data_core_persistence::{OutboxRepository, WorkflowRepository};
use r_data_core_services::workflow::outbox::{
    enqueue_workflow_push_outbox, DispatchWorkflowOutboxBatchUseCase, EnqueueWorkflowFetchUseCase,
    WorkflowOutboxDispatcher,
};
use r_data_core_services::WorkflowRepositoryAdapter;
use r_data_core_test_support::create_test_admin_user;
use r_data_core_workflow::data::job_queue::JobQueue;
use r_data_core_workflow::data::jobs::{FetchAndStageJob, ProcessRawItemJob, SendEmailJob};
use r_data_core_workflow::data::WorkflowKind;
use sqlx::postgres::PgListener;
use sqlx::Row;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use uuid::Uuid;

async fn maybe_setup_test_db() -> Option<r_data_core_test_support::TestDatabase> {
    let pool = r_data_core_test_support::try_setup_test_db().await;
    if pool.is_none() {
        eprintln!("Skipping test: test database not available");
    }
    pool
}

#[derive(Clone)]
struct RecordingQueue {
    fetches: Arc<Mutex<Vec<FetchAndStageJob>>>,
    fail_fetch: Arc<AtomicBool>,
    permanent_fetch_failure: Arc<AtomicBool>,
}

impl RecordingQueue {
    fn new() -> Self {
        Self {
            fetches: Arc::new(Mutex::new(Vec::new())),
            fail_fetch: Arc::new(AtomicBool::new(false)),
            permanent_fetch_failure: Arc::new(AtomicBool::new(false)),
        }
    }

    fn with_fetch_failure() -> Self {
        Self {
            fetches: Arc::new(Mutex::new(Vec::new())),
            fail_fetch: Arc::new(AtomicBool::new(true)),
            permanent_fetch_failure: Arc::new(AtomicBool::new(false)),
        }
    }

    fn with_permanent_fetch_failure() -> Self {
        Self {
            fetches: Arc::new(Mutex::new(Vec::new())),
            fail_fetch: Arc::new(AtomicBool::new(true)),
            permanent_fetch_failure: Arc::new(AtomicBool::new(true)),
        }
    }

    async fn fetch_count(&self) -> usize {
        self.fetches.lock().await.len()
    }
}

#[async_trait::async_trait]
impl JobQueue for RecordingQueue {
    async fn enqueue_fetch(&self, job: FetchAndStageJob) -> r_data_core_core::error::Result<()> {
        if self.fail_fetch.load(Ordering::SeqCst) {
            if self.permanent_fetch_failure.load(Ordering::SeqCst) {
                return Err(r_data_core_core::error::Error::Config(
                    "synthetic queue configuration failure".to_string(),
                ));
            }
            return Err(r_data_core_core::error::Error::Unknown(
                "synthetic queue failure".to_string(),
            ));
        }
        self.fetches.lock().await.push(job);
        Ok(())
    }

    async fn enqueue_process(
        &self,
        _job: ProcessRawItemJob,
    ) -> r_data_core_core::error::Result<()> {
        Ok(())
    }

    async fn enqueue_email(&self, _job: SendEmailJob) -> r_data_core_core::error::Result<()> {
        Ok(())
    }

    async fn blocking_pop_email(&self) -> r_data_core_core::error::Result<SendEmailJob> {
        Err(r_data_core_core::error::Error::Unknown(
            "email queue not implemented for RecordingQueue".to_string(),
        ))
    }
}

async fn create_consumer_workflow(
    workflow_repo: &WorkflowRepository,
    creator_uuid: Uuid,
    name_prefix: &str,
) -> anyhow::Result<Uuid> {
    workflow_repo
        .create(
            &r_data_core_workflow::data::requests::CreateWorkflowRequest {
                name: format!("{name_prefix}-{}", Uuid::now_v7().simple()),
                description: Some("outbox test".into()),
                kind: WorkflowKind::Consumer.to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({
                    "steps": [
                        {
                            "from": {
                                "type": "format",
                                "source": {
                                    "source_type": "uri",
                                    "config": { "uri": "http://example.com/data.csv" }
                                },
                                "format": {
                                    "format_type": "csv",
                                    "options": {}
                                },
                                "mapping": {}
                            },
                            "transform": { "type": "none" },
                            "to": {
                                "type": "format",
                                "output": { "mode": "api" },
                                "format": {
                                    "format_type": "json",
                                    "options": {}
                                },
                                "mapping": {}
                            }
                        }
                    ]
                }),
                versioning_disabled: false,
            },
            creator_uuid,
        )
        .await
        .map_err(Into::into)
}

#[tokio::test]
async fn enqueue_fetch_use_case_routes_directly_when_fetch_outbox_disabled() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };

    let creator_uuid = create_test_admin_user(&pool).await?;
    let workflow_repo = WorkflowRepository::new(pool.pool.clone());
    let workflow_uuid =
        create_consumer_workflow(&workflow_repo, creator_uuid, "fetch-direct").await?;

    let repo_adapter: Arc<dyn WorkflowRepositoryTrait> = Arc::new(WorkflowRepositoryAdapter::new(
        WorkflowRepository::new(pool.pool.clone()),
    ));
    let outbox_repo = OutboxRepository::new(pool.pool.clone());
    let queue = RecordingQueue::new();

    let use_case =
        EnqueueWorkflowFetchUseCase::new(&repo_adapter, &queue, Some(&outbox_repo), None, false);
    let run_uuid = use_case.enqueue_run_for_fetch(workflow_uuid, None).await?;

    assert_eq!(
        queue.fetch_count().await,
        1,
        "fetch should be enqueued directly"
    );

    let fetch_outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM outbox_messages WHERE topic = $1 AND aggregate_id = $2",
    )
    .bind(r_data_core_core::outbox::WORKFLOW_FETCH_TOPIC)
    .bind(run_uuid.to_string())
    .fetch_one(&pool.pool)
    .await?;
    assert_eq!(
        fetch_outbox_count, 0,
        "no fetch outbox row should be written when disabled"
    );

    Ok(())
}

#[tokio::test]
async fn enqueue_fetch_use_case_uses_outbox_when_fetch_outbox_enabled() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };

    let creator_uuid = create_test_admin_user(&pool).await?;
    let workflow_repo = WorkflowRepository::new(pool.pool.clone());
    let workflow_uuid =
        create_consumer_workflow(&workflow_repo, creator_uuid, "fetch-outbox").await?;

    let repo_adapter: Arc<dyn WorkflowRepositoryTrait> = Arc::new(WorkflowRepositoryAdapter::new(
        WorkflowRepository::new(pool.pool.clone()),
    ));
    let outbox_repo = OutboxRepository::new(pool.pool.clone());
    let queue = RecordingQueue::new();

    let use_case =
        EnqueueWorkflowFetchUseCase::new(&repo_adapter, &queue, Some(&outbox_repo), None, true);
    let run_uuid = use_case.enqueue_run_for_fetch(workflow_uuid, None).await?;

    assert_eq!(
        queue.fetch_count().await,
        1,
        "fetch should be delivered through outbox dispatcher"
    );

    let fetch_outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM outbox_messages WHERE topic = $1 AND aggregate_id = $2",
    )
    .bind(r_data_core_core::outbox::WORKFLOW_FETCH_TOPIC)
    .bind(run_uuid.to_string())
    .fetch_one(&pool.pool)
    .await?;
    assert_eq!(
        fetch_outbox_count, 1,
        "fetch outbox row should be written when enabled"
    );

    Ok(())
}

#[tokio::test]
async fn insert_run_queued_with_fetch_outbox_persists_claimable_message() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let creator_uuid = create_test_admin_user(&pool).await?;

    let workflow_repo = WorkflowRepository::new(pool.pool.clone());
    let workflow_uuid = workflow_repo
        .create(
            &r_data_core_workflow::data::requests::CreateWorkflowRequest {
                name: format!("outbox-test-{}", Uuid::now_v7().simple()),
                description: Some("outbox test".into()),
                kind: WorkflowKind::Consumer.to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({
                    "steps": [
                        {
                            "from": {
                                "type": "format",
                                "source": {
                                    "source_type": "uri",
                                    "config": { "uri": "http://example.com/data.csv" }
                                },
                                "format": {
                                    "format_type": "csv",
                                    "options": {}
                                },
                                "mapping": {}
                            },
                            "transform": { "type": "none" },
                            "to": {
                                "type": "format",
                                "output": { "mode": "api" },
                                "format": {
                                    "format_type": "json",
                                    "options": {}
                                },
                                "mapping": {}
                            }
                        }
                    ]
                }),
                versioning_disabled: false,
            },
            creator_uuid,
        )
        .await?;

    let trigger_id = Uuid::now_v7();
    let (run_uuid, outbox_uuid) = workflow_repo
        .insert_run_queued_with_fetch_outbox(workflow_uuid, trigger_id)
        .await?;

    let outbox_repo = OutboxRepository::new(pool.pool.clone());
    let row = sqlx::query(
        r"
        SELECT status::text AS status, topic, kind, aggregate_type, aggregate_id, payload, headers
        FROM outbox_messages
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .fetch_one(&pool.pool)
    .await?;

    let status: String = row.try_get("status")?;
    let topic: String = row.try_get("topic")?;
    let kind: String = row.try_get("kind")?;
    let aggregate_type: String = row.try_get("aggregate_type")?;
    let aggregate_id: String = row.try_get("aggregate_id")?;
    let payload: serde_json::Value = row.try_get("payload")?;
    let headers: serde_json::Value = row.try_get("headers")?;

    assert_eq!(status, "pending");
    assert_eq!(topic, r_data_core_core::outbox::WORKFLOW_FETCH_TOPIC);
    assert_eq!(kind, r_data_core_core::outbox::WORKFLOW_FETCH_ENQUEUE_KIND);
    assert_eq!(aggregate_type, "workflow_run");
    assert_eq!(aggregate_id, run_uuid.to_string());
    assert_eq!(payload["workflow_id"], serde_json::json!(workflow_uuid));
    assert_eq!(payload["trigger_id"], serde_json::json!(run_uuid));
    assert_eq!(headers["run_uuid"], serde_json::json!(run_uuid));

    let claimed = outbox_repo.claim_due(10, "repo-test-worker").await?;
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].uuid, outbox_uuid);
    assert_eq!(claimed[0].status, "processing");

    outbox_repo
        .mark_delivered(outbox_uuid, Some("repo-test-worker"))
        .await?;
    let delivered_status: String =
        sqlx::query_scalar("SELECT status::text FROM outbox_messages WHERE uuid = $1")
            .bind(outbox_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(delivered_status, "delivered");

    Ok(())
}

#[tokio::test]
async fn outbox_status_updates_respect_claim_owner() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let workflow_uuid = Uuid::now_v7();
    let run_uuid = Uuid::now_v7();
    let item_uuid = Uuid::now_v7();
    let payload = serde_json::json!({
        "workflow_id": workflow_uuid,
        "run_uuid": run_uuid,
        "item_uuid": item_uuid,
    });
    let headers = serde_json::json!({
        "workflow_id": workflow_uuid,
        "run_uuid": run_uuid,
        "item_uuid": item_uuid,
    });
    let outbox_uuid = outbox_repo
        .insert_workflow_push_enqueue(
            workflow_uuid,
            run_uuid,
            item_uuid,
            payload,
            headers,
            "guard-test",
        )
        .await?;

    sqlx::query(
        r"
        UPDATE outbox_messages
        SET status = 'processing',
            locked_at = NOW(),
            locked_by = 'expected-worker'
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .execute(&pool.pool)
    .await?;

    let wrong_owner_result = outbox_repo
        .mark_delivered(outbox_uuid, Some("different-worker"))
        .await;
    assert!(wrong_owner_result.is_err());

    let status_after_wrong_owner: String =
        sqlx::query_scalar("SELECT status::text FROM outbox_messages WHERE uuid = $1")
            .bind(outbox_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(status_after_wrong_owner, "processing");

    let unowned_retry_result = outbox_repo
        .mark_retry(
            outbox_uuid,
            "retry should not apply without a lock owner",
            OffsetDateTime::now_utc(),
            None,
        )
        .await;
    assert!(unowned_retry_result.is_err());

    let status_after_unowned_retry: String =
        sqlx::query_scalar("SELECT status::text FROM outbox_messages WHERE uuid = $1")
            .bind(outbox_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(status_after_unowned_retry, "processing");

    outbox_repo
        .mark_delivered(outbox_uuid, Some("expected-worker"))
        .await?;

    let status_after_expected_owner: String =
        sqlx::query_scalar("SELECT status::text FROM outbox_messages WHERE uuid = $1")
            .bind(outbox_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(status_after_expected_owner, "delivered");

    Ok(())
}

#[tokio::test]
async fn workflow_push_outbox_dispatches_http_request_and_marks_delivered() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let server = MockServer::start_async().await;
    let push_mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/push");
            then.status(204);
        })
        .await;

    let workflow_uuid = Uuid::now_v7();
    let run_uuid = Uuid::now_v7();
    let item_uuid = Uuid::now_v7();
    let data = serde_json::json!({"hello": "world"});
    let data_bytes = serde_json::to_vec(&data)?;
    let outbox_uuid = enqueue_workflow_push_outbox(
        &outbox_repo,
        workflow_uuid,
        run_uuid,
        item_uuid,
        "uri",
        serde_json::json!({ "uri": server.url("/push") }),
        None,
        Some(r_data_core_workflow::data::adapters::destination::HttpMethod::Post),
        "json",
        &data_bytes,
    )
    .await?;

    let claimed = outbox_repo.claim_due(10, "push-test-worker").await?;
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].uuid, outbox_uuid);

    WorkflowOutboxDispatcher::new(None, &outbox_repo, Some("push-test-worker"), None)
        .dispatch_push_record(&claimed[0])
        .await?;

    push_mock.assert_async().await;
    let status: String =
        sqlx::query_scalar("SELECT status::text FROM outbox_messages WHERE uuid = $1")
            .bind(outbox_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(status, "delivered");

    Ok(())
}

#[tokio::test]
async fn workflow_outbox_insert_emits_database_notification() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let mut listener = PgListener::connect_with(&pool.pool).await?;
    listener
        .listen(r_data_core_core::outbox::WORKFLOW_OUTBOX_NOTIFY_CHANNEL)
        .await?;

    let workflow_uuid = Uuid::now_v7();
    let run_uuid = Uuid::now_v7();
    let item_uuid = Uuid::now_v7();
    let data_bytes = serde_json::to_vec(&serde_json::json!({"hello": "notify"}))?;

    let _ = enqueue_workflow_push_outbox(
        &outbox_repo,
        workflow_uuid,
        run_uuid,
        item_uuid,
        "uri",
        serde_json::json!({ "uri": "http://example.com/push" }),
        None,
        Some(r_data_core_workflow::data::adapters::destination::HttpMethod::Post),
        "json",
        &data_bytes,
    )
    .await?;

    let notification = tokio::time::timeout(Duration::from_secs(5), listener.recv()).await??;
    assert_eq!(
        notification.channel(),
        r_data_core_core::outbox::WORKFLOW_OUTBOX_NOTIFY_CHANNEL
    );

    Ok(())
}

#[tokio::test]
async fn workflow_outbox_terminal_cleanup_removes_old_delivered_rows() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let workflow_uuid = Uuid::now_v7();
    let run_uuid = Uuid::now_v7();
    let item_uuid = Uuid::now_v7();
    let payload = serde_json::json!({"workflow_id": workflow_uuid, "run_uuid": run_uuid, "item_uuid": item_uuid});
    let headers = serde_json::json!({"workflow_id": workflow_uuid, "run_uuid": run_uuid, "item_uuid": item_uuid});

    let outbox_uuid = outbox_repo
        .insert_workflow_push_enqueue(
            workflow_uuid,
            run_uuid,
            item_uuid,
            payload,
            headers,
            "cleanup-test",
        )
        .await?;

    sqlx::query(
        r"
        UPDATE outbox_messages
        SET status = 'delivered',
            processed_at = NOW() - INTERVAL '31 days'
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .execute(&pool.pool)
    .await?;

    let cutoff = OffsetDateTime::now_utc() - time::Duration::days(30);
    let removed = outbox_repo.purge_terminal_older_than(cutoff).await?;
    assert_eq!(removed, 1);

    let remaining: Option<String> =
        sqlx::query_scalar("SELECT status::text FROM outbox_messages WHERE uuid = $1")
            .bind(outbox_uuid)
            .fetch_optional(&pool.pool)
            .await?;
    assert!(remaining.is_none());

    Ok(())
}

#[tokio::test]
async fn workflow_push_outbox_retries_on_http_error() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let server = MockServer::start_async().await;
    let push_mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/push");
            then.status(500);
        })
        .await;

    let workflow_uuid = Uuid::now_v7();
    let run_uuid = Uuid::now_v7();
    let item_uuid = Uuid::now_v7();
    let data_bytes = serde_json::to_vec(&serde_json::json!({"hello": "retry"}))?;
    let outbox_uuid = enqueue_workflow_push_outbox(
        &outbox_repo,
        workflow_uuid,
        run_uuid,
        item_uuid,
        "uri",
        serde_json::json!({ "uri": server.url("/push") }),
        None,
        Some(r_data_core_workflow::data::adapters::destination::HttpMethod::Post),
        "json",
        &data_bytes,
    )
    .await?;

    let claimed = outbox_repo.claim_due(10, "push-retry-worker").await?;
    assert_eq!(claimed.len(), 1);
    WorkflowOutboxDispatcher::new(None, &outbox_repo, Some("push-retry-worker"), None)
        .dispatch_push_record(&claimed[0])
        .await?;

    push_mock.assert_async().await;
    let row = sqlx::query(
        r"
        SELECT status::text AS status, attempt_count, last_error, available_at
        FROM outbox_messages
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .fetch_one(&pool.pool)
    .await?;

    let status: String = row.try_get("status")?;
    let attempt_count: i32 = row.try_get("attempt_count")?;
    let last_error: Option<String> = row.try_get("last_error")?;
    assert_eq!(status, "retry");
    assert_eq!(attempt_count, 1);
    assert!(last_error.as_deref().is_some_and(|msg| msg.contains("500")));

    Ok(())
}

#[tokio::test]
async fn workflow_outbox_worker_reclaims_stale_processing_rows() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let creator_uuid = create_test_admin_user(&pool).await?;
    let workflow_repo = WorkflowRepository::new(pool.pool.clone());
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let workflow_uuid = workflow_repo
        .create(
            &r_data_core_workflow::data::requests::CreateWorkflowRequest {
                name: format!("outbox-reclaim-{}", Uuid::now_v7().simple()),
                description: Some("outbox reclaim test".into()),
                kind: WorkflowKind::Consumer.to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({
                    "steps": [
                        {
                            "from": {
                                "type": "format",
                                "source": {
                                    "source_type": "uri",
                                    "config": { "uri": "http://example.com/data.csv" }
                                },
                                "format": {
                                    "format_type": "csv",
                                    "options": {}
                                },
                                "mapping": {}
                            },
                            "transform": { "type": "none" },
                            "to": {
                                "type": "format",
                                "output": { "mode": "api" },
                                "format": {
                                    "format_type": "json",
                                    "options": {}
                                },
                                "mapping": {}
                            }
                        }
                    ]
                }),
                versioning_disabled: false,
            },
            creator_uuid,
        )
        .await?;

    let trigger_id = Uuid::now_v7();
    let (run_uuid, outbox_uuid) = workflow_repo
        .insert_run_queued_with_fetch_outbox(workflow_uuid, trigger_id)
        .await?;

    sqlx::query(
        r"
        UPDATE outbox_messages
        SET status = 'processing',
            locked_at = NOW() - INTERVAL '10 minutes',
            locked_by = 'stale-worker'
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .execute(&pool.pool)
    .await?;

    let queue = RecordingQueue::new();
    let dispatched = DispatchWorkflowOutboxBatchUseCase::new(
        &queue,
        &outbox_repo,
        "reclaim-worker",
        10,
        300,
        None,
    )
    .run_once()
    .await?;
    assert_eq!(dispatched, 1);
    assert_eq!(queue.fetch_count().await, 1);

    let status: String =
        sqlx::query_scalar("SELECT status::text FROM outbox_messages WHERE uuid = $1")
            .bind(outbox_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(status, "delivered");

    let run_status = workflow_repo.get_run_status(run_uuid).await?;
    assert_eq!(run_status.as_deref(), Some("queued"));

    Ok(())
}

#[tokio::test]
async fn workflow_outbox_worker_retries_when_queue_enqueue_fails() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let creator_uuid = create_test_admin_user(&pool).await?;
    let workflow_repo = WorkflowRepository::new(pool.pool.clone());
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let workflow_uuid = workflow_repo
        .create(
            &r_data_core_workflow::data::requests::CreateWorkflowRequest {
                name: format!("outbox-retry-{}", Uuid::now_v7().simple()),
                description: Some("outbox retry test".into()),
                kind: WorkflowKind::Consumer.to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({
                    "steps": [
                        {
                            "from": {
                                "type": "format",
                                "source": {
                                    "source_type": "uri",
                                    "config": { "uri": "http://example.com/data.csv" }
                                },
                                "format": {
                                    "format_type": "csv",
                                    "options": {}
                                },
                                "mapping": {}
                            },
                            "transform": { "type": "none" },
                            "to": {
                                "type": "format",
                                "output": { "mode": "api" },
                                "format": {
                                    "format_type": "json",
                                    "options": {}
                                },
                                "mapping": {}
                            }
                        }
                    ]
                }),
                versioning_disabled: false,
            },
            creator_uuid,
        )
        .await?;

    let trigger_id = Uuid::now_v7();
    let (_run_uuid, outbox_uuid) = workflow_repo
        .insert_run_queued_with_fetch_outbox(workflow_uuid, trigger_id)
        .await?;

    let queue = RecordingQueue::with_fetch_failure();
    let dispatched = DispatchWorkflowOutboxBatchUseCase::new(
        &queue,
        &outbox_repo,
        "retry-worker",
        10,
        300,
        None,
    )
    .run_once()
    .await?;
    assert_eq!(dispatched, 1);

    let row = sqlx::query(
        r"
        SELECT status::text AS status, attempt_count, last_error
        FROM outbox_messages
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .fetch_one(&pool.pool)
    .await?;

    let status: String = row.try_get("status")?;
    let attempt_count: i32 = row.try_get("attempt_count")?;
    let last_error: Option<String> = row.try_get("last_error")?;
    assert_eq!(status, "retry");
    assert_eq!(attempt_count, 1);
    assert!(last_error
        .as_deref()
        .is_some_and(|msg| msg.contains("synthetic queue failure")));

    Ok(())
}

#[tokio::test]
async fn workflow_outbox_worker_dead_letters_permanent_queue_failures() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let creator_uuid = create_test_admin_user(&pool).await?;
    let workflow_repo = WorkflowRepository::new(pool.pool.clone());
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let workflow_uuid = workflow_repo
        .create(
            &r_data_core_workflow::data::requests::CreateWorkflowRequest {
                name: format!("outbox-permanent-failure-{}", Uuid::now_v7().simple()),
                description: Some("outbox permanent failure test".into()),
                kind: WorkflowKind::Consumer.to_string(),
                enabled: true,
                schedule_cron: None,
                config: serde_json::json!({
                    "steps": [
                        {
                            "from": {
                                "type": "format",
                                "source": {
                                    "source_type": "uri",
                                    "config": { "uri": "http://example.com/data.csv" }
                                },
                                "format": {
                                    "format_type": "csv",
                                    "options": {}
                                },
                                "mapping": {}
                            },
                            "transform": { "type": "none" },
                            "to": {
                                "type": "format",
                                "output": { "mode": "api" },
                                "format": {
                                    "format_type": "json",
                                    "options": {}
                                },
                                "mapping": {}
                            }
                        }
                    ]
                }),
                versioning_disabled: false,
            },
            creator_uuid,
        )
        .await?;

    let trigger_id = Uuid::now_v7();
    let (_run_uuid, outbox_uuid) = workflow_repo
        .insert_run_queued_with_fetch_outbox(workflow_uuid, trigger_id)
        .await?;

    let queue = RecordingQueue::with_permanent_fetch_failure();
    let dispatched = DispatchWorkflowOutboxBatchUseCase::new(
        &queue,
        &outbox_repo,
        "permanent-worker",
        10,
        300,
        None,
    )
    .run_once()
    .await?;
    assert_eq!(dispatched, 1);

    let row = sqlx::query(
        r"
        SELECT status::text AS status, attempt_count, last_error
        FROM outbox_messages
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .fetch_one(&pool.pool)
    .await?;

    let status: String = row.try_get("status")?;
    let attempt_count: i32 = row.try_get("attempt_count")?;
    let last_error: Option<String> = row.try_get("last_error")?;
    assert_eq!(status, "dead_letter");
    assert_eq!(attempt_count, 1);
    assert!(last_error
        .as_deref()
        .is_some_and(|msg| msg.contains("synthetic queue configuration failure")));

    Ok(())
}

#[tokio::test]
async fn workflow_push_outbox_dead_letters_on_client_error() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let server = MockServer::start_async().await;
    let push_mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/push");
            then.status(404);
        })
        .await;

    let workflow_uuid = Uuid::now_v7();
    let run_uuid = Uuid::now_v7();
    let item_uuid = Uuid::now_v7();
    let data_bytes = serde_json::to_vec(&serde_json::json!({"hello": "dead-letter"}))?;
    let outbox_uuid = enqueue_workflow_push_outbox(
        &outbox_repo,
        workflow_uuid,
        run_uuid,
        item_uuid,
        "uri",
        serde_json::json!({ "uri": server.url("/push") }),
        None,
        Some(r_data_core_workflow::data::adapters::destination::HttpMethod::Post),
        "json",
        &data_bytes,
    )
    .await?;

    let claimed = outbox_repo.claim_due(10, "push-dead-letter-worker").await?;
    assert_eq!(claimed.len(), 1);
    WorkflowOutboxDispatcher::new(None, &outbox_repo, Some("push-dead-letter-worker"), None)
        .dispatch_push_record(&claimed[0])
        .await?;

    push_mock.assert_async().await;
    let row = sqlx::query(
        r"
        SELECT status::text AS status, attempt_count, last_error
        FROM outbox_messages
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .fetch_one(&pool.pool)
    .await?;

    let status: String = row.try_get("status")?;
    let attempt_count: i32 = row.try_get("attempt_count")?;
    let last_error: Option<String> = row.try_get("last_error")?;
    assert_eq!(status, "dead_letter");
    assert_eq!(attempt_count, 1);
    assert!(last_error.as_deref().is_some_and(|msg| msg.contains("404")));

    Ok(())
}

#[tokio::test]
async fn workflow_push_outbox_retries_on_rate_limit_error() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let server = MockServer::start_async().await;
    let push_mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/push");
            then.status(429);
        })
        .await;

    let workflow_uuid = Uuid::now_v7();
    let run_uuid = Uuid::now_v7();
    let item_uuid = Uuid::now_v7();
    let data_bytes = serde_json::to_vec(&serde_json::json!({"hello": "rate-limit"}))?;
    let outbox_uuid = enqueue_workflow_push_outbox(
        &outbox_repo,
        workflow_uuid,
        run_uuid,
        item_uuid,
        "uri",
        serde_json::json!({ "uri": server.url("/push") }),
        None,
        Some(r_data_core_workflow::data::adapters::destination::HttpMethod::Post),
        "json",
        &data_bytes,
    )
    .await?;

    let claimed = outbox_repo.claim_due(10, "push-429-worker").await?;
    assert_eq!(claimed.len(), 1);
    WorkflowOutboxDispatcher::new(None, &outbox_repo, Some("push-429-worker"), None)
        .dispatch_push_record(&claimed[0])
        .await?;

    push_mock.assert_async().await;
    let row = sqlx::query(
        r"
        SELECT status::text AS status, attempt_count, last_error
        FROM outbox_messages
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .fetch_one(&pool.pool)
    .await?;

    let status: String = row.try_get("status")?;
    let attempt_count: i32 = row.try_get("attempt_count")?;
    let last_error: Option<String> = row.try_get("last_error")?;
    assert_eq!(status, "retry");
    assert_eq!(attempt_count, 1);
    assert!(last_error.as_deref().is_some_and(|msg| msg.contains("429")));

    Ok(())
}

#[tokio::test]
async fn workflow_push_outbox_dead_letters_invalid_auth_config() -> anyhow::Result<()> {
    let Some(pool) = maybe_setup_test_db().await else {
        return Ok(());
    };
    let outbox_repo = OutboxRepository::new(pool.pool.clone());

    let workflow_uuid = Uuid::now_v7();
    let run_uuid = Uuid::now_v7();
    let item_uuid = Uuid::now_v7();
    let data_bytes = serde_json::to_vec(&serde_json::json!({"hello": "world"}))?;
    let outbox_uuid = enqueue_workflow_push_outbox(
        &outbox_repo,
        workflow_uuid,
        run_uuid,
        item_uuid,
        "uri",
        serde_json::json!({ "uri": "http://example.com/push" }),
        Some(serde_json::json!({ "type": "bearer" })),
        Some(r_data_core_workflow::data::adapters::destination::HttpMethod::Post),
        "json",
        &data_bytes,
    )
    .await?;

    let claimed = outbox_repo.claim_due(10, "push-auth-worker").await?;
    assert_eq!(claimed.len(), 1);
    WorkflowOutboxDispatcher::new(None, &outbox_repo, Some("push-auth-worker"), None)
        .dispatch_push_record(&claimed[0])
        .await?;

    let row = sqlx::query(
        r"
        SELECT status::text AS status, attempt_count, last_error
        FROM outbox_messages
        WHERE uuid = $1
        ",
    )
    .bind(outbox_uuid)
    .fetch_one(&pool.pool)
    .await?;

    let status: String = row.try_get("status")?;
    let attempt_count: i32 = row.try_get("attempt_count")?;
    let last_error: Option<String> = row.try_get("last_error")?;
    assert_eq!(status, "dead_letter");
    assert_eq!(attempt_count, 1);
    assert!(last_error
        .as_deref()
        .is_some_and(|msg| msg.contains("Invalid workflow push auth configuration")));

    Ok(())
}
