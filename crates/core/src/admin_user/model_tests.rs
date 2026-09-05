#![allow(clippy::unwrap_used)]

use super::model::*;
use crate::permissions::role::{AccessLevel, Permission, PermissionType, ResourceNamespace, Role};

#[test]
fn test_admin_user_has_permission_superadmin() {
    let user = AdminUserBuilder::new(
        "admin".to_string(),
        "admin@test.com".to_string(),
        "hash".to_string(),
        "Admin User".to_string(),
        UserStatus::Active,
        true, // super_admin flag
        "Admin".to_string(),
        "User".to_string(),
        true,
        true,
    )
    .build();

    let roles = vec![];

    // Super admin should have all permissions
    assert!(user.has_permission(
        &roles,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None
    ));
    assert!(user.has_permission(
        &roles,
        &ResourceNamespace::Entities,
        &PermissionType::Delete,
        Some("/any/path")
    ));
}

#[test]
fn test_admin_user_has_permission_with_role() {
    let user = AdminUserBuilder::new(
        "user".to_string(),
        "user@test.com".to_string(),
        "hash".to_string(),
        "Test User".to_string(),
        UserStatus::Active,
        false, // super_admin flag
        "Test".to_string(),
        "User".to_string(),
        true,
        false,
    )
    .build();

    let mut role = Role::new("MyRole".to_string());
    role.add_permission(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    })
    .unwrap();

    let roles = vec![role];

    // User has read permission
    assert!(user.has_permission(
        &roles,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None
    ));

    // User does not have create permission
    assert!(!user.has_permission(
        &roles,
        &ResourceNamespace::Workflows,
        &PermissionType::Create,
        None
    ));
}

#[test]
fn test_admin_user_has_permission_with_super_admin_role() {
    let user = AdminUserBuilder::new(
        "user".to_string(),
        "user@test.com".to_string(),
        "hash".to_string(),
        "Test User".to_string(),
        UserStatus::Active,
        false, // super_admin flag
        "Test".to_string(),
        "User".to_string(),
        true,
        false,
    )
    .build();

    let mut role = Role::new("SuperAdminRole".to_string());
    role.super_admin = true;
    let roles = vec![role];

    // User with super_admin role should have all permissions
    assert!(user.has_permission(
        &roles,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None
    ));
    assert!(user.has_permission(
        &roles,
        &ResourceNamespace::Entities,
        &PermissionType::Delete,
        Some("/any/path")
    ));
}

fn active_user() -> AdminUser {
    AdminUserBuilder::new(
        "lockme".to_string(),
        "lockme@test.com".to_string(),
        "hash".to_string(),
        "Lock Me".to_string(),
        UserStatus::Active,
        false,
        "Lock".to_string(),
        "Me".to_string(),
        true,
        false,
    )
    .build()
}

#[test]
fn lockout_triggers_at_the_configured_threshold() {
    let mut user = active_user();

    for _ in 0..2 {
        user.record_login_failure(3, 900);
        assert!(user.can_login());
    }

    user.record_login_failure(3, 900);
    assert_eq!(user.status, UserStatus::Locked);
    assert!(!user.can_login());
    assert!(user.locked_until.is_some());
}

#[test]
fn zero_duration_locks_until_an_operator_intervenes() {
    let mut user = active_user();
    user.record_login_failure(1, 0);

    assert_eq!(user.status, UserStatus::Locked);
    assert!(user.locked_until.is_none());
    assert!(!user.release_expired_lockout());
}

#[test]
fn expired_lockout_releases_itself() {
    let mut user = active_user();
    user.record_login_failure(1, 900);
    assert!(!user.can_login());

    // Still inside the window.
    assert!(!user.release_expired_lockout());

    user.locked_until = Some(time::OffsetDateTime::now_utc() - time::Duration::seconds(1));
    assert!(user.release_expired_lockout());
    assert!(user.can_login());
    assert_eq!(user.failed_login_attempts, 0);
    // A second call has nothing left to do.
    assert!(!user.release_expired_lockout());
}

#[test]
fn successful_login_clears_lockout_state() {
    let mut user = active_user();
    user.record_login_failure(5, 900);
    user.record_login_success();

    assert_eq!(user.failed_login_attempts, 0);
    assert!(user.locked_until.is_none());
    assert!(user.last_login.is_some());
}

#[test]
fn setting_status_active_unlocks_the_account() {
    let mut user = active_user();
    user.record_login_failure(1, 900);
    assert_eq!(user.status, UserStatus::Locked);

    user.set_status(UserStatus::Active);
    assert!(user.can_login());
    assert_eq!(user.failed_login_attempts, 0);
    assert!(user.locked_until.is_none());
}

#[test]
fn setting_status_inactive_keeps_the_account_out() {
    let mut user = active_user();
    user.set_status(UserStatus::Inactive);

    assert!(!user.can_login());
    assert!(user.locked_until.is_none());
}
