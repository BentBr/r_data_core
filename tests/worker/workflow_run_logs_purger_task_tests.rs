#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::error::Result;
use r_data_core_core::maintenance::MaintenanceTask;
use r_data_core_core::settings::WorkflowRunLogSettings;
use r_data_core_services::SettingsService;
use r_data_core_test_support::{create_test_admin_user, setup_test_db};
use r_data_core_worker::context::TaskContext;
use r_data_core_worker::tasks::workflow_run_logs_purger::WorkflowRunLogsPurgerTask;
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

fn make_cache() -> Arc<CacheManager> {
    Arc::new(CacheManager::new(CacheConfig {
        entity_definition_ttl: 3600,
        api_key_ttl: 600,
        enabled: true,
        ttl: 3600,
        max_size: 10000,
    }))
}

/// Insert a dummy workflow so foreign key constraints are satisfied
async fn insert_workflow(pool: &sqlx::PgPool, workflow_uuid: Uuid, user_uuid: Uuid) {
    sqlx::query(
        "INSERT INTO workflows (uuid, name, kind, config, created_by)
         VALUES ($1, $2, 'consumer'::workflow_kind, '{}'::jsonb, $3)
         ON CONFLICT (uuid) DO NOTHING",
    )
    .bind(workflow_uuid)
    .bind(format!("test-workflow-{workflow_uuid}"))
    .bind(user_uuid)
    .execute(pool)
    .await
    .expect("insert workflow");
}

/// Insert a workflow run with a given status and age
async fn insert_run(pool: &sqlx::PgPool, workflow_uuid: Uuid, status: &str, age_days: i32) -> Uuid {
    let run_uuid = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO workflow_runs (uuid, workflow_uuid, status, queued_at)
         VALUES ($1, $2, $3::workflow_run_status, NOW() - make_interval(days => $4))",
    )
    .bind(run_uuid)
    .bind(workflow_uuid)
    .bind(status)
    .bind(age_days)
    .execute(pool)
    .await
    .expect("insert workflow run");
    run_uuid
}

#[tokio::test]
#[serial]
async fn test_task_name_and_cron() -> Result<()> {
    let task = WorkflowRunLogsPurgerTask::new("0 0 * * * *".to_string());

    assert_eq!(task.name(), "workflow_run_logs_purger");
    assert_eq!(task.cron(), "0 0 * * * *");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_skips_when_disabled() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let cache_manager = make_cache();
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    // Disable pruning
    let settings = WorkflowRunLogSettings {
        enabled: false,
        max_runs: Some(5),
        max_age_days: Some(30),
    };
    settings_service
        .update_workflow_run_log_settings(&settings, user_uuid)
        .await?;

    // Execute the task - should skip when disabled
    let task = WorkflowRunLogsPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_prunes_by_age() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let cache_manager = make_cache();
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    let workflow_uuid = Uuid::now_v7();
    insert_workflow(&pool.pool, workflow_uuid, user_uuid).await;

    // Enable pruning with age-based pruning (30 days)
    let settings = WorkflowRunLogSettings {
        enabled: true,
        max_runs: None,
        max_age_days: Some(30),
    };
    settings_service
        .update_workflow_run_log_settings(&settings, user_uuid)
        .await?;

    // Create runs: one old (40 days), one recent (10 days)
    let _old_run = insert_run(&pool.pool, workflow_uuid, "success", 40).await;
    let recent_run = insert_run(&pool.pool, workflow_uuid, "success", 10).await;

    // Verify both runs exist before pruning
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_runs WHERE workflow_uuid = $1")
            .bind(workflow_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(count_before, 2, "Should have 2 runs before pruning");

    // Execute the task
    let task = WorkflowRunLogsPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // Verify old run is removed, recent run remains
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_runs WHERE workflow_uuid = $1")
            .bind(workflow_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(count_after, 1, "Should have 1 run after age-based pruning");

    // Verify the remaining run is the recent one
    let remaining: Uuid =
        sqlx::query_scalar("SELECT uuid FROM workflow_runs WHERE workflow_uuid = $1")
            .bind(workflow_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(
        remaining, recent_run,
        "Remaining run should be the recent one"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_prunes_by_count() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let cache_manager = make_cache();
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    let workflow_uuid = Uuid::now_v7();
    insert_workflow(&pool.pool, workflow_uuid, user_uuid).await;

    // Enable pruning with count-based pruning (keep latest 2)
    let settings = WorkflowRunLogSettings {
        enabled: true,
        max_runs: Some(2),
        max_age_days: None,
    };
    settings_service
        .update_workflow_run_log_settings(&settings, user_uuid)
        .await?;

    // Create 5 runs with increasing age so ordering is deterministic
    for i in 1..=5 {
        insert_run(&pool.pool, workflow_uuid, "success", i * 2).await;
    }

    // Verify all 5 runs exist before pruning
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_runs WHERE workflow_uuid = $1")
            .bind(workflow_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(count_before, 5, "Should have 5 runs before pruning");

    // Execute the task
    let task = WorkflowRunLogsPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // Verify only 2 runs remain (latest 2)
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_runs WHERE workflow_uuid = $1")
            .bind(workflow_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(
        count_after, 2,
        "Should have 2 runs after count-based pruning"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_prunes_by_both_age_and_count() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let cache_manager = make_cache();
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    let workflow_uuid = Uuid::now_v7();
    insert_workflow(&pool.pool, workflow_uuid, user_uuid).await;

    // Enable pruning with both age and count
    let settings = WorkflowRunLogSettings {
        enabled: true,
        max_runs: Some(3),
        max_age_days: Some(30),
    };
    settings_service
        .update_workflow_run_log_settings(&settings, user_uuid)
        .await?;

    // Create 2 old runs (40 days) and 3 recent runs (10 days)
    for _ in 0..2 {
        insert_run(&pool.pool, workflow_uuid, "success", 40).await;
    }
    for _ in 0..3 {
        insert_run(&pool.pool, workflow_uuid, "success", 10).await;
    }

    // Verify all 5 runs exist before pruning
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_runs WHERE workflow_uuid = $1")
            .bind(workflow_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(count_before, 5, "Should have 5 runs before pruning");

    // Execute the task
    let task = WorkflowRunLogsPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // After age pruning: 2 old removed, 3 recent remain
    // After count pruning: keep latest 3, all 3 remain
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_runs WHERE workflow_uuid = $1")
            .bind(workflow_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(
        count_after, 3,
        "Should have 3 runs after both pruning operations"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_with_no_runs() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let cache_manager = make_cache();
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    // Enable pruning
    let settings = WorkflowRunLogSettings {
        enabled: true,
        max_runs: Some(5),
        max_age_days: Some(30),
    };
    settings_service
        .update_workflow_run_log_settings(&settings, user_uuid)
        .await?;

    // Execute the task with no runs - should not error
    let task = WorkflowRunLogsPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_does_not_prune_running_or_queued() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let cache_manager = make_cache();
    let settings_service = SettingsService::new(pool.pool.clone(), cache_manager.clone());

    let workflow_uuid = Uuid::now_v7();
    insert_workflow(&pool.pool, workflow_uuid, user_uuid).await;

    // Enable pruning with aggressive settings
    let settings = WorkflowRunLogSettings {
        enabled: true,
        max_runs: Some(1),
        max_age_days: Some(1),
    };
    settings_service
        .update_workflow_run_log_settings(&settings, user_uuid)
        .await?;

    // Create old runs with various statuses
    let queued_run = insert_run(&pool.pool, workflow_uuid, "queued", 10).await;
    let running_run = insert_run(&pool.pool, workflow_uuid, "running", 10).await;
    let _success_run = insert_run(&pool.pool, workflow_uuid, "success", 10).await;
    let _failed_run = insert_run(&pool.pool, workflow_uuid, "failed", 10).await;

    // Verify all 4 runs exist before pruning
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_runs WHERE workflow_uuid = $1")
            .bind(workflow_uuid)
            .fetch_one(&pool.pool)
            .await?;
    assert_eq!(count_before, 4, "Should have 4 runs before pruning");

    // Execute the task
    let task = WorkflowRunLogsPurgerTask::new("0 0 * * * *".to_string());
    let context = TaskContext::with_cache(pool.pool.clone(), cache_manager);
    task.execute(&context)
        .await
        .map_err(|e| r_data_core_core::error::Error::Config(e.to_string()))?;

    // Queued and running runs must be preserved
    let remaining_uuids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT uuid FROM workflow_runs WHERE workflow_uuid = $1 ORDER BY queued_at",
    )
    .bind(workflow_uuid)
    .fetch_all(&pool.pool)
    .await?;

    assert!(
        remaining_uuids.contains(&queued_run),
        "Queued run should be preserved"
    );
    assert!(
        remaining_uuids.contains(&running_run),
        "Running run should be preserved"
    );

    Ok(())
}
