#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Integration tests verifying that service CRUD operations write audit entries to `system_logs`.

use r_data_core_core::cache::CacheManager;
use r_data_core_core::config::CacheConfig;
use r_data_core_core::entity_definition::definition::{EntityDefinition, EntityDefinitionParams};
use r_data_core_core::field::types::FieldType;
use r_data_core_core::field::ui::UiSettings;
use r_data_core_core::field::{FieldDefinition, FieldValidation};
use r_data_core_core::permissions::role::{
    AccessLevel, Permission, PermissionType, ResourceNamespace, Role,
};
use r_data_core_core::system_log::{SystemLog, SystemLogResourceType, SystemLogType};
use r_data_core_persistence::{
    AdminUserRepository, ApiKeyRepository, EntityDefinitionRepository, SystemLogRepository,
    SystemLogRepositoryTrait, WorkflowRepository,
};
use r_data_core_services::{
    AdminUserRepositoryAdapter, AdminUserService, ApiKeyRepositoryAdapter, ApiKeyService,
    EntityDefinitionRepositoryAdapter, EntityDefinitionService, RoleService, SystemLogService,
    WorkflowRepositoryAdapter, WorkflowService,
};
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use r_data_core_workflow::data::requests::CreateWorkflowRequest;
use serial_test::serial;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `SystemLogService` backed by the real repository.
fn setup_system_log(pool: &sqlx::PgPool) -> Arc<SystemLogService> {
    let repo = SystemLogRepository::new(pool.clone());
    Arc::new(SystemLogService::new(Arc::new(repo)))
}

/// Fetch the most-recently-inserted system log entry for a specific resource type.
async fn get_latest_log_for_resource(
    pool: &sqlx::PgPool,
    resource_type: SystemLogResourceType,
) -> Option<SystemLog> {
    let repo = SystemLogRepository::new(pool.clone());
    let (logs, _) = repo
        .list_paginated(
            1,
            0,
            &r_data_core_persistence::SystemLogFilter {
                resource_type: Some(resource_type),
                ..Default::default()
            },
        )
        .await
        .ok()?;
    logs.into_iter().next()
}

/// Build a minimal `CacheManager` with caching disabled.
fn disabled_cache() -> Arc<CacheManager> {
    let config = CacheConfig {
        enabled: false,
        ttl: 3600,
        max_size: 10_000,
        entity_definition_ttl: 0,
        api_key_ttl: 600,
    };
    Arc::new(CacheManager::new(config))
}

/// Build a minimal valid `CreateWorkflowRequest`.
fn minimal_workflow_request(name: &str) -> CreateWorkflowRequest {
    CreateWorkflowRequest {
        name: name.to_string(),
        description: None,
        kind: "consumer".to_string(),
        enabled: true,
        schedule_cron: None,
        versioning_disabled: false,
        config: serde_json::json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" }
                    },
                    "format": { "format_type": "csv", "options": {} },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": { "format_type": "json", "options": {} },
                    "mapping": {}
                }
            }]
        }),
    }
}

/// Build a minimal `EntityDefinition` for testing.
fn minimal_entity_definition(entity_type: &str, created_by: Uuid) -> EntityDefinition {
    EntityDefinition::from_params(EntityDefinitionParams {
        entity_type: entity_type.to_string(),
        display_name: format!("{entity_type} Display"),
        description: Some("audit test".to_string()),
        group_name: None,
        allow_children: false,
        icon: None,
        fields: vec![FieldDefinition {
            name: "title".to_string(),
            display_name: "Title".to_string(),
            description: None,
            field_type: FieldType::String,
            required: true,
            indexed: false,
            filterable: false,
            unique: false,
            default_value: None,
            validation: FieldValidation::default(),
            ui_settings: UiSettings::default(),
            constraints: HashMap::new(),
        }],
        created_by,
    })
}

/// Build a minimal test `Role`.
fn minimal_role(name: &str) -> Role {
    let mut role = Role::new(name.to_string());
    role.description = Some("audit test role".to_string());
    role.permissions = vec![Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    }];
    role
}

// ---------------------------------------------------------------------------
// AdminUser tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_admin_user_create_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let service = AdminUserService::new(Arc::new(AdminUserRepositoryAdapter::new(repo)))
        .with_system_log(system_log);

    let actor = create_test_admin_user(&pool.pool).await.expect("actor");
    let uuid = service
        .register_user(
            "auditcreateuser",
            "auditcreate@example.com",
            "Password123!",
            "Audit",
            "Create",
            None,
            true,
            actor,
        )
        .await
        .expect("register_user");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::AdminUser)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityCreated);
    assert_eq!(log.resource_type, SystemLogResourceType::AdminUser);
    assert_eq!(log.resource_uuid, Some(uuid));
    assert!(
        log.summary.contains("auditcreateuser"),
        "summary should mention the username"
    );
}

#[tokio::test]
#[serial]
async fn test_admin_user_delete_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let repo = AdminUserRepository::new(Arc::new(pool.pool.clone()));
    let service = AdminUserService::new(Arc::new(AdminUserRepositoryAdapter::new(repo)))
        .with_system_log(system_log);

    let actor = create_test_admin_user(&pool.pool).await.expect("actor");
    // Create a user to delete
    let user_uuid = service
        .register_user(
            "auditdeleteuser",
            "auditdelete@example.com",
            "Password123!",
            "Audit",
            "Delete",
            None,
            true,
            actor,
        )
        .await
        .expect("register_user");

    service
        .delete_user(&user_uuid, actor)
        .await
        .expect("delete_user");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::AdminUser)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityDeleted);
    assert_eq!(log.resource_type, SystemLogResourceType::AdminUser);
    assert_eq!(log.resource_uuid, Some(user_uuid));
    assert!(
        log.summary.contains("auditdeleteuser"),
        "summary should mention the username"
    );
}

// ---------------------------------------------------------------------------
// Role tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_role_create_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let service =
        RoleService::new(pool.pool.clone(), disabled_cache(), None).with_system_log(system_log);

    let actor = create_test_admin_user(&pool.pool).await.expect("actor");
    let role = minimal_role("audit-create-role");
    let uuid = service
        .create_role(&role, actor)
        .await
        .expect("create_role");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::Role)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityCreated);
    assert_eq!(log.resource_type, SystemLogResourceType::Role);
    assert_eq!(log.resource_uuid, Some(uuid));
    assert!(
        log.summary.contains("audit-create-role"),
        "summary should mention the role name"
    );
}

#[tokio::test]
#[serial]
async fn test_role_delete_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let service =
        RoleService::new(pool.pool.clone(), disabled_cache(), None).with_system_log(system_log);

    let actor = create_test_admin_user(&pool.pool).await.expect("actor");
    let role = minimal_role("audit-delete-role");
    let uuid = service
        .create_role(&role, actor)
        .await
        .expect("create_role");

    service.delete_role(uuid, actor).await.expect("delete_role");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::Role)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityDeleted);
    assert_eq!(log.resource_type, SystemLogResourceType::Role);
    assert_eq!(log.resource_uuid, Some(uuid));
    assert!(
        log.summary.contains("audit-delete-role"),
        "summary should mention the role name"
    );
}

// ---------------------------------------------------------------------------
// Workflow tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_workflow_create_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let repo = WorkflowRepository::new(pool.pool.clone());
    let adapter = WorkflowRepositoryAdapter::new(repo);
    let service = WorkflowService::new(Arc::new(adapter)).with_system_log(system_log);

    let actor = create_test_admin_user(&pool.pool).await.expect("actor");
    let req = minimal_workflow_request("audit-create-wf");
    let uuid = service.create(&req, actor).await.expect("create workflow");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::Workflow)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityCreated);
    assert_eq!(log.resource_type, SystemLogResourceType::Workflow);
    assert_eq!(log.resource_uuid, Some(uuid));
    assert!(
        log.summary.contains("audit-create-wf"),
        "summary should mention the workflow name"
    );
}

#[tokio::test]
#[serial]
async fn test_workflow_delete_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let repo = WorkflowRepository::new(pool.pool.clone());
    let adapter = WorkflowRepositoryAdapter::new(repo);
    let service = WorkflowService::new(Arc::new(adapter)).with_system_log(system_log);

    let actor = create_test_admin_user(&pool.pool).await.expect("actor");
    let req = minimal_workflow_request("audit-delete-wf");
    let uuid = service.create(&req, actor).await.expect("create workflow");

    service.delete(uuid, actor).await.expect("delete workflow");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::Workflow)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityDeleted);
    assert_eq!(log.resource_type, SystemLogResourceType::Workflow);
    assert_eq!(log.resource_uuid, Some(uuid));
    assert!(
        log.summary.contains("audit-delete-wf"),
        "summary should mention the workflow name"
    );
}

// ---------------------------------------------------------------------------
// EntityDefinition tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_entity_definition_create_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let repo = EntityDefinitionRepository::new(pool.pool.clone());
    let service = EntityDefinitionService::new_without_cache(Arc::new(
        EntityDefinitionRepositoryAdapter::new(repo),
    ))
    .with_system_log(system_log);

    let creator = create_test_admin_user(&pool.pool).await.expect("creator");
    let def = minimal_entity_definition("auditcreatetype", creator);
    let uuid = service
        .create_entity_definition(&def)
        .await
        .expect("create_entity_definition");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::EntityDefinition)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityCreated);
    assert_eq!(log.resource_type, SystemLogResourceType::EntityDefinition);
    assert_eq!(log.resource_uuid, Some(uuid));
    assert!(
        log.summary.contains("auditcreatetype"),
        "summary should mention the entity type"
    );

    // Cleanup: delete the entity definition so the schema doesn't linger
    let _ = service.delete_entity_definition(&uuid, creator).await;
}

#[tokio::test]
#[serial]
async fn test_entity_definition_delete_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let repo = EntityDefinitionRepository::new(pool.pool.clone());
    let service = EntityDefinitionService::new_without_cache(Arc::new(
        EntityDefinitionRepositoryAdapter::new(repo),
    ))
    .with_system_log(system_log);

    let creator = create_test_admin_user(&pool.pool).await.expect("creator");
    let def = minimal_entity_definition("auditdeletetype", creator);
    let uuid = service
        .create_entity_definition(&def)
        .await
        .expect("create_entity_definition");

    service
        .delete_entity_definition(&uuid, creator)
        .await
        .expect("delete_entity_definition");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::EntityDefinition)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityDeleted);
    assert_eq!(log.resource_type, SystemLogResourceType::EntityDefinition);
    assert_eq!(log.resource_uuid, Some(uuid));
    assert!(
        log.summary.contains("auditdeletetype"),
        "summary should mention the entity type"
    );
}

// ---------------------------------------------------------------------------
// ApiKey tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_api_key_create_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let service = ApiKeyService::new(Arc::new(ApiKeyRepositoryAdapter::new(repo)))
        .with_system_log(system_log);

    let user_uuid = create_test_admin_user(&pool.pool).await.expect("user");
    let (key_uuid, _key_value) = service
        .create_api_key("audit-create-key", "audit test key", user_uuid, 30)
        .await
        .expect("create_api_key");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::ApiKey)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityCreated);
    assert_eq!(log.resource_type, SystemLogResourceType::ApiKey);
    assert_eq!(log.resource_uuid, Some(key_uuid));
    assert!(
        log.summary.contains("audit-create-key"),
        "summary should mention the key name"
    );
}

#[tokio::test]
#[serial]
async fn test_api_key_revoke_logs_to_system_log() {
    let pool = setup_test_db().await;
    clear_test_db(&pool).await.expect("clear_test_db failed");

    let system_log = setup_system_log(&pool.pool);
    let repo = ApiKeyRepository::new(Arc::new(pool.pool.clone()));
    let service = ApiKeyService::new(Arc::new(ApiKeyRepositoryAdapter::new(repo)))
        .with_system_log(system_log);

    let user_uuid = create_test_admin_user(&pool.pool).await.expect("user");
    let (key_uuid, _key_value) = service
        .create_api_key("audit-revoke-key", "audit test revoke key", user_uuid, 30)
        .await
        .expect("create_api_key");

    service
        .revoke_key(key_uuid, user_uuid)
        .await
        .expect("revoke_key");

    let log = get_latest_log_for_resource(&pool.pool, SystemLogResourceType::ApiKey)
        .await
        .expect("log entry must exist");

    assert_eq!(log.log_type, SystemLogType::EntityDeleted);
    assert_eq!(log.resource_type, SystemLogResourceType::ApiKey);
    assert_eq!(log.resource_uuid, Some(key_uuid));
    assert!(
        log.summary.contains("audit-revoke-key"),
        "summary should mention the key name"
    );
}
