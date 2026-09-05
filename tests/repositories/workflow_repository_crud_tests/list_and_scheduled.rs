#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for `list_all`, `list_paginated`, and `list_scheduled_consumers`.

use super::{seed, seed_with_cron};
use r_data_core_core::error::Result;
use r_data_core_persistence::WorkflowRepository;
use r_data_core_test_support::{
    clear_test_db, create_test_admin_user, random_string, setup_test_db,
};
use serial_test::serial;

// ── list_all ordering ─────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_list_all_ordered_by_name() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    // Insert names in reverse alpha order
    seed(&repo, user, "zzz-workflow", "consumer").await?;
    seed(&repo, user, "aaa-workflow", "consumer").await?;
    seed(&repo, user, "mmm-workflow", "consumer").await?;

    let all = repo.list_all().await?;
    let names: Vec<&str> = all.iter().map(|w| w.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(
        names, sorted,
        "list_all must return rows ordered by name ASC"
    );

    Ok(())
}

// ── list_paginated: DESC sort ─────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_list_paginated_sort_by_name_desc() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    seed(&repo, user, "alpha-workflow", "consumer").await?;
    seed(&repo, user, "beta-workflow", "consumer").await?;
    seed(&repo, user, "gamma-workflow", "consumer").await?;

    let page = repo
        .list_paginated(10, 0, Some("name".to_string()), Some("DESC".to_string()))
        .await?;

    assert_eq!(page.len(), 3);
    // DESC: gamma > beta > alpha
    assert_eq!(page[0].name, "gamma-workflow");
    assert_eq!(page[2].name, "alpha-workflow");

    Ok(())
}

// ── list_paginated: explicit ASC ──────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn test_list_paginated_sort_by_name_asc_explicit() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    seed(&repo, user, "zebra-workflow", "consumer").await?;
    seed(&repo, user, "ant-workflow", "consumer").await?;

    let page = repo
        .list_paginated(10, 0, Some("name".to_string()), Some("ASC".to_string()))
        .await?;

    assert_eq!(page.len(), 2);
    assert_eq!(page[0].name, "ant-workflow");
    assert_eq!(page[1].name, "zebra-workflow");

    Ok(())
}

// ── list_paginated: i64::MAX limit (no-LIMIT SQL path) ───────────────────────

#[tokio::test]
#[serial]
async fn test_list_paginated_max_limit_returns_all() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    for i in 0..4 {
        seed(&repo, user, &format!("max-lim-{i}"), "consumer").await?;
    }

    let all = repo.list_paginated(i64::MAX, 0, None, None).await?;
    assert_eq!(all.len(), 4);

    Ok(())
}

// ── list_paginated: invalid sort_order defaults to ASC ───────────────────────

#[tokio::test]
#[serial]
async fn test_list_paginated_invalid_sort_order_defaults_to_asc() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    seed(&repo, user, "bb-workflow", "consumer").await?;
    seed(&repo, user, "aa-workflow", "consumer").await?;

    let page = repo
        .list_paginated(10, 0, Some("name".to_string()), Some("INVALID".to_string()))
        .await?;

    assert_eq!(page.len(), 2);
    // invalid order → defaults to ASC
    assert_eq!(page[0].name, "aa-workflow");

    Ok(())
}

// ── list_scheduled_consumers: from.api source excluded ───────────────────────

#[tokio::test]
#[serial]
async fn test_list_scheduled_consumers_excludes_from_api_source() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    // Config with from source_type = "api", no endpoint → excluded
    let api_config = serde_json::json!({
        "steps": [{
            "from": {
                "source": {
                    "source_type": "api",
                    "config": {}
                }
            }
        }]
    });
    let api_uuid = seed_with_cron(&repo, user, &random_string("api-src"), api_config, true).await?;

    // Normal consumer — should appear
    let normal_uuid = seed_with_cron(
        &repo,
        user,
        &random_string("normal-cron"),
        serde_json::json!({}),
        true,
    )
    .await?;

    let scheduled = repo.list_scheduled_consumers().await?;

    assert!(
        !scheduled.iter().any(|(u, _)| *u == api_uuid),
        "workflow with from.api source must be excluded"
    );
    assert!(
        scheduled.iter().any(|(u, _)| *u == normal_uuid),
        "normal cron workflow must be included"
    );

    Ok(())
}

// ── list_scheduled_consumers: to.format output=api excluded ──────────────────

#[tokio::test]
#[serial]
async fn test_list_scheduled_consumers_excludes_to_format_output_api() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    let to_api_config = serde_json::json!({
        "steps": [{"to": {"type": "format", "output": "api"}}]
    });
    let to_api_uuid =
        seed_with_cron(&repo, user, &random_string("to-api"), to_api_config, true).await?;

    let scheduled = repo.list_scheduled_consumers().await?;

    assert!(
        !scheduled.iter().any(|(u, _)| *u == to_api_uuid),
        "workflow with to.format output=api must be excluded"
    );

    Ok(())
}

// ── list_scheduled_consumers: to.format output.mode=api excluded ─────────────

#[tokio::test]
#[serial]
async fn test_list_scheduled_consumers_excludes_to_format_output_mode_api() -> Result<()> {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await?;

    let repo = WorkflowRepository::new(pool.pool.clone());
    let user = create_test_admin_user(&pool).await?;

    let mode_api_config = serde_json::json!({
        "steps": [{"to": {"type": "format", "output": {"mode": "api"}}}]
    });
    let mode_api_uuid = seed_with_cron(
        &repo,
        user,
        &random_string("mode-api"),
        mode_api_config,
        true,
    )
    .await?;

    let scheduled = repo.list_scheduled_consumers().await?;
    assert!(
        !scheduled.iter().any(|(u, _)| *u == mode_api_uuid),
        "workflow with to.format output.mode=api must be excluded"
    );

    Ok(())
}
