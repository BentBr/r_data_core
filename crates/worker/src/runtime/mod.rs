#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::sync::Arc;

use futures::future;
use log::{debug, error, info, warn};
use tokio::sync::Mutex;
use tokio_cron_scheduler::JobScheduler;
use uuid::Uuid;

use r_data_core_core::config::load_worker_config;
use r_data_core_persistence::{ComponentVersionRepository, WorkflowRepository};
use r_data_core_services::bootstrap::{init_cache_manager, init_logger_with_default, init_pg_pool};
use r_data_core_services::LicenseService;
use r_data_core_workflow::data::job_queue::apalis_redis::ApalisRedisQueue;

pub mod consumer;
pub mod outbox;
pub mod scheduler;

use crate::runtime::consumer::spawn_consumer_loop;
use crate::runtime::outbox::spawn_outbox_recovery_loop;
use crate::runtime::scheduler::start_scheduler;

/// Current version from Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) struct WorkerRuntime {
    pub(crate) pool: sqlx::PgPool,
    pub(crate) queue: Arc<ApalisRedisQueue>,
    pub(crate) queue_fetch_key: String,
    pub(crate) cache_manager: Arc<r_data_core_core::cache::CacheManager>,
    pub(crate) outbox_repo: Option<Arc<r_data_core_persistence::OutboxRepository>>,
    pub(crate) outbox_retry_policy:
        Option<r_data_core_services::workflow::outbox::OutboxRetryPolicy>,
    pub(crate) job_queue_update_interval_secs: u64,
    pub(crate) outbox_stale_lease_secs: i64,
    pub(crate) outbox_db_url: String,
    pub(crate) jwt_secret: Option<String>,
    pub(crate) jwt_expiration: u64,
}

pub(crate) struct WorkerBootstrap {
    pub(crate) runtime: WorkerRuntime,
    pub(crate) repo: WorkflowRepository,
    pub(crate) scheduler: JobScheduler,
    pub(crate) scheduled_workflows: Arc<Mutex<std::collections::HashMap<Uuid, (Uuid, String)>>>,
}

/// Run the worker process.
///
/// # Errors
///
/// Returns an error if the worker cannot initialize its runtime dependencies.
pub async fn run() -> r_data_core_core::error::Result<()> {
    init_logger_with_default("info");
    info!("Starting data workflow worker");

    let bootstrap = bootstrap_worker().await?;
    let _scheduler = start_scheduler(&bootstrap).await?;

    spawn_consumer_loop(&bootstrap.runtime);
    spawn_outbox_recovery_loop(&bootstrap.runtime);

    // Park forever.
    future::pending::<()>().await;
    Ok(())
}

async fn bootstrap_worker() -> r_data_core_core::error::Result<WorkerBootstrap> {
    let config = match load_worker_config() {
        Ok(cfg) => {
            debug!("Loaded conf: {cfg:?}");
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {e}");
            panic!("Failed to load configuration: {e}");
        }
    };

    let pool = init_pg_pool(
        &config.database.connection_string,
        config.database.max_connections,
    )
    .await
    .map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to initialize database pool: {e}"))
    })?;

    let cache_manager =
        init_cache_manager(config.cache.clone(), Some(&config.queue.redis_url)).await;
    let license_service = LicenseService::new(config.license.clone(), cache_manager.clone());
    license_service.verify_license_on_startup("worker").await;

    let version_repo = ComponentVersionRepository::new(pool.clone());
    if let Err(e) = version_repo.upsert("worker", VERSION).await {
        warn!("Failed to report worker version: {e}");
    } else {
        info!("Worker version {VERSION} registered");
    }

    let queue_cfg = Arc::new(config.queue.clone());
    let queue = Arc::new(
        ApalisRedisQueue::from_parts(
            &queue_cfg.redis_url,
            &queue_cfg.fetch_key,
            &queue_cfg.process_key,
        )
        .await?,
    );
    let outbox_repo = if config.outbox_enabled {
        Some(Arc::new(r_data_core_persistence::OutboxRepository::new(
            pool.clone(),
        )))
    } else {
        None
    };
    let outbox_retry_policy = if config.outbox_enabled {
        Some(
            r_data_core_services::workflow::outbox::OutboxRetryPolicy::new(
                config.outbox_retry_base_delay_secs,
                config.outbox_retry_multiplier,
                config.outbox_retry_max_delay_secs,
            ),
        )
    } else {
        None
    };

    let runtime = WorkerRuntime {
        pool: pool.clone(),
        queue,
        queue_fetch_key: queue_cfg.fetch_key.clone(),
        cache_manager,
        outbox_repo,
        outbox_retry_policy,
        job_queue_update_interval_secs: config.job_queue_update_interval_secs,
        outbox_stale_lease_secs: config.outbox_stale_lease_secs,
        outbox_db_url: config.database.connection_string,
        jwt_secret: std::env::var("JWT_SECRET").ok(),
        jwt_expiration: std::env::var("JWT_EXPIRATION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(86_400),
    };

    let scheduler = JobScheduler::new().await.map_err(|e| {
        r_data_core_core::error::Error::Config(format!("Failed to create job scheduler: {e}"))
    })?;

    Ok(WorkerBootstrap {
        runtime,
        repo: WorkflowRepository::new(pool),
        scheduler,
        scheduled_workflows: Arc::new(Mutex::new(std::collections::HashMap::new())),
    })
}
