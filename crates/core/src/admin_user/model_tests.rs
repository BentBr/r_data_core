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
