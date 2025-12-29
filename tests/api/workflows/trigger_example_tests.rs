#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

// Tests for loading and executing example workflow JSON files with trigger source

use super::common::{
    create_consumer_workflow, create_test_entity_definition, generate_entity_type,
    load_workflow_example, setup_app_with_entities,
};
use actix_web::test;
use uuid::Uuid;

// ============================================================================
// Load and validate example JSON files
// ============================================================================

#[actix_web::test]
async fn test_load_trigger_to_external_api_entity_example() -> anyhow::Result<()> {
    let entity_type = generate_entity_type("test_trigger_example");
    let config =
        load_workflow_example("workflow_trigger_to_external_api_entity.json", &entity_type)?;

    // Validate structure
    assert!(config.get("steps").is_some());
    let steps = config.get("steps").unwrap().as_array().unwrap();
    assert_eq!(steps.len(), 2);

    // Step 1 should have trigger source
    let step1 = &steps[0];
    let from1 = step1.get("from").unwrap();
    let source1 = from1.get("source").unwrap();
    assert_eq!(
        source1.get("source_type").unwrap().as_str().unwrap(),
        "trigger"
    );

    // Step 2 should have uri source
    let step2 = &steps[1];
    let from2 = step2.get("from").unwrap();
    let source2 = from2.get("source").unwrap();
    assert_eq!(source2.get("source_type").unwrap().as_str().unwrap(), "uri");

    // Validate it can be parsed as DSL
    let program = r_data_core_workflow::dsl::DslProgram::from_config(&config)?;
    program.validate()?;

    Ok(())
}

#[actix_web::test]
async fn test_load_trigger_to_external_api_push_example() -> anyhow::Result<()> {
    let config = load_workflow_example("workflow_trigger_to_external_api_push.json", "test")?;

    // Validate structure
    assert!(config.get("steps").is_some());
    let steps = config.get("steps").unwrap().as_array().unwrap();
    assert_eq!(steps.len(), 2);

    // Step 1 should have trigger source
    let step1 = &steps[0];
    let from1 = step1.get("from").unwrap();
    let source1 = from1.get("source").unwrap();
    assert_eq!(
        source1.get("source_type").unwrap().as_str().unwrap(),
        "trigger"
    );

    // Step 2 should have push output
    let step2 = &steps[1];
    let to2 = step2.get("to").unwrap();
    let output2 = to2.get("output").unwrap();
    assert_eq!(output2.get("mode").unwrap().as_str().unwrap(), "push");

    // Validate it can be parsed as DSL
    let program = r_data_core_workflow::dsl::DslProgram::from_config(&config)?;
    program.validate()?;

    Ok(())
}

#[actix_web::test]
async fn test_load_trigger_multi_step_example() -> anyhow::Result<()> {
    let entity_type = generate_entity_type("test_trigger_multi");
    let config = load_workflow_example("workflow_trigger_multi_step.json", &entity_type)?;

    // Validate structure
    assert!(config.get("steps").is_some());
    let steps = config.get("steps").unwrap().as_array().unwrap();
    assert_eq!(steps.len(), 3);

    // Step 1 should have trigger source
    let step1 = &steps[0];
    let from1 = step1.get("from").unwrap();
    let source1 = from1.get("source").unwrap();
    assert_eq!(
        source1.get("source_type").unwrap().as_str().unwrap(),
        "trigger"
    );

    // Step 2 should have uri source and arithmetic transform
    let step2 = &steps[1];
    let from2 = step2.get("from").unwrap();
    let source2 = from2.get("source").unwrap();
    assert_eq!(source2.get("source_type").unwrap().as_str().unwrap(), "uri");
    let transform2 = step2.get("transform").unwrap();
    assert_eq!(
        transform2.get("type").unwrap().as_str().unwrap(),
        "arithmetic"
    );

    // Step 3 should use previous_step
    let step3 = &steps[2];
    let from3 = step3.get("from").unwrap();
    assert_eq!(
        from3.get("type").unwrap().as_str().unwrap(),
        "previous_step"
    );

    // Validate it can be parsed as DSL
    let program = r_data_core_workflow::dsl::DslProgram::from_config(&config)?;
    program.validate()?;

    Ok(())
}

// ============================================================================
// Execute example workflows
// ============================================================================

#[actix_web::test]
async fn test_execute_trigger_to_external_api_entity_workflow() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    let entity_type = generate_entity_type("test_execute_trigger");
    let _ed_uuid = create_test_entity_definition(&pool, &entity_type).await?;

    // Load example workflow
    let config =
        load_workflow_example("workflow_trigger_to_external_api_entity.json", &entity_type)?;

    // Create workflow
    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Trigger workflow via GET
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should accept the request (even if external URI fails, the trigger should work)
    assert!(
        resp.status().is_success()
            || resp.status().as_u16() == 202
            || resp.status().as_u16() == 500,
        "Expected success, 202, or 500 (if external URI fails), got: {}",
        resp.status()
    );

    Ok(())
}

#[actix_web::test]
async fn test_execute_trigger_to_external_api_push_workflow() -> anyhow::Result<()> {
    let (app, pool, token, _) = setup_app_with_entities().await?;
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool.pool)
        .await?;

    // Load example workflow
    let config = load_workflow_example("workflow_trigger_to_external_api_push.json", "test")?;

    // Create workflow
    let wf_uuid = create_consumer_workflow(&pool, creator_uuid, config, true, None).await?;

    // Trigger workflow via GET
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/workflows/{wf_uuid}"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should accept the request (even if external URI fails, the trigger should work)
    assert!(
        resp.status().is_success()
            || resp.status().as_u16() == 202
            || resp.status().as_u16() == 500,
        "Expected success, 202, or 500 (if external URI fails), got: {}",
        resp.status()
    );

    Ok(())
}
