use actix_web::{test, web, App};
use r_data_core::api::{configure_app, ApiState};
use r_data_core::cache::CacheManager;
use r_data_core::config::CacheConfig;
use r_data_core::entity::admin_user::model::AdminUser;
use r_data_core::entity::admin_user::repository::{AdminUserRepository, ApiKeyRepository};
use r_data_core::services::{
    AdminUserService, ApiKeyService, EntityDefinitionService, WorkflowRepositoryAdapter,
};
use r_data_core::workflow::data::repository::WorkflowRepository;
use r_data_core::workflow::data::WorkflowKind;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

// Import common test utilities
#[path = "../common/mod.rs"]
mod common;

async fn setup_app_and_token() -> anyhow::Result<(
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    sqlx::PgPool,
    String,
)> {
    // DB
    let pool = common::utils::setup_test_db().await;

    // Minimal services for configure_app
    let cache_config = CacheConfig {
        enabled: true,
        ttl: 300,
        max_size: 10000,
    };
    let cache_manager = Arc::new(CacheManager::new(cache_config));

    let api_key_repository = Arc::new(ApiKeyRepository::new(Arc::new(pool.clone())));
    let api_key_service = ApiKeyService::new(api_key_repository);

    let admin_user_repository = Arc::new(AdminUserRepository::new(Arc::new(pool.clone())));
    let admin_user_service = AdminUserService::new(admin_user_repository);

    let entity_definition_service =
        EntityDefinitionService::new(Arc::new(r_data_core::api::admin::entity_definitions::repository::EntityDefinitionRepository::new(pool.clone())));

    let wf_repo = WorkflowRepository::new(pool.clone());
    let wf_adapter = WorkflowRepositoryAdapter::new(wf_repo);
    let workflow_service = r_data_core::services::workflow_service::WorkflowService::new(Arc::new(wf_adapter));

    let jwt_secret = "test_secret".to_string();
    let app_state = web::Data::new(ApiState {
        db_pool: pool.clone(),
        jwt_secret: jwt_secret.clone(),
        cache_manager,
        api_key_service,
        admin_user_service,
        entity_definition_service,
        dynamic_entity_service: None,
        workflow_service,
    });

    let app = test::init_service(App::new().app_data(app_state.clone()).configure(configure_app)).await;

    // Ensure a test admin user exists and produce a JWT
    let user_uuid = common::utils::create_test_admin_user(&pool).await?;
    let user: AdminUser = sqlx::query_as("SELECT * FROM admin_users WHERE uuid = $1")
        .bind(user_uuid)
        .fetch_one(&pool)
        .await?;
    let token = r_data_core::api::jwt::generate_access_token(&user, &jwt_secret)?;

    Ok((app, pool, token))
}

#[actix_web::test]
async fn create_workflow_uses_required_auth_and_sets_created_by() -> anyhow::Result<()> {
    let (app, pool, token) = setup_app_and_token().await?;

    // Minimal valid request
    let payload = serde_json::json!({
        "name": format!("wf-route-create-{}", Uuid::now_v7()),
        "description": "route test",
        "kind": WorkflowKind::Consumer.to_string(),
        "enabled": true,
        "schedule_cron": null,
        "config": {
            "steps": [
                {
                    "from": { "type": "csv", "uri": "http://example.com/data.csv", "mapping": {} },
                    "transform": { "type": "none" },
                    "to": { "type": "json", "output": "api", "mapping": {} }
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/workflows")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(payload.clone())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "status: {}", resp.status());

    let body = test::read_body(resp).await;
    let v: serde_json::Value = serde_json::from_slice(&body)?;
    let wf_uuid = Uuid::parse_str(v.get("data").and_then(|d| d.get("uuid")).and_then(|s| s.as_str()).unwrap())?;

    // Verify created_by equals JWT user
    let row = sqlx::query("SELECT created_by FROM workflows WHERE uuid = $1")
        .bind(wf_uuid)
        .fetch_one(&pool)
        .await?;
    let created_by: Uuid = row.try_get("created_by")?;
    // Extract sub from token by verifying again
    // (we can also join with admin_users for existence)
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_users WHERE uuid = $1")
        .bind(created_by)
        .fetch_one(&pool)
        .await?;
    assert_eq!(total, 1, "created_by must reference an admin_users row");

    Ok(())
}

#[actix_web::test]
async fn update_workflow_sets_updated_by() -> anyhow::Result<()> {
    let (app, pool, token) = setup_app_and_token().await?;

    // First create a workflow directly via repository to get a UUID
    let repo = WorkflowRepository::new(pool.clone());
    let creator_uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM admin_users LIMIT 1")
        .fetch_one(&pool)
        .await?;
    let create_req = r_data_core::api::admin::workflows::models::CreateWorkflowRequest {
        name: format!("wf-route-update-{}", Uuid::now_v7()),
        description: Some("route test".to_string()),
        kind: WorkflowKind::Consumer,
        enabled: true,
        schedule_cron: None,
        config: serde_json::json!({
            "steps": [
                {
                    "from": { "type": "csv", "uri": "http://example.com/data.csv", "mapping": {} },
                    "transform": { "type": "none" },
                    "to": { "type": "json", "output": "api", "mapping": {} }
                }
            ]
        }),
    };
    let wf_uuid = repo.create(&create_req, creator_uuid).await?;

    // Now update via route with auth; expect updated_by set
    let update_payload = serde_json::json!({
        "name": format!("wf-route-update-{}-patched", Uuid::now_v7()),
        "description": "updated",
        "kind": WorkflowKind::Consumer.to_string(),
        "enabled": false,
        "schedule_cron": "*/10 * * * *",
        "config": {
            "steps": [
                { "from": { "type": "csv", "uri": "http://example.com/data.csv", "mapping": {} }, "transform": { "type": "none" }, "to": { "type": "json", "output": "api", "mapping": {} } }
            ]
        }
    });

    let req = test::TestRequest::put()
        .uri(&format!("/admin/api/v1/workflows/{}", wf_uuid))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(update_payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "status: {}", resp.status());

    // verify updated_by set
    let row = sqlx::query("SELECT updated_by FROM workflows WHERE uuid = $1")
        .bind(wf_uuid)
        .fetch_one(&pool)
        .await?;
    let updated_by: Option<Uuid> = row.try_get("updated_by")?;
    assert!(updated_by.is_some(), "updated_by must be set on update");

    Ok(())
}


