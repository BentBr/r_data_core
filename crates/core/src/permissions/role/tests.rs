#![allow(clippy::unwrap_used)]

use super::*;

fn create_test_role() -> Role {
    let mut role = Role::new("Test Role".to_string());

    // Add workflow permissions
    role.add_permission(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    })
    .unwrap();

    role.add_permission(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Create,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    })
    .unwrap();

    // Add entity permissions with path constraint
    role.add_permission(Permission {
        resource_type: ResourceNamespace::Entities,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: Some(serde_json::json!({"path": "/projects"})),
    })
    .unwrap();

    role.add_permission(Permission {
        resource_type: ResourceNamespace::Entities,
        permission_type: PermissionType::Delete,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: Some(serde_json::json!({"path": "/projects"})),
    })
    .unwrap();

    role
}

#[test]
fn test_has_permission_namespace() {
    let role = create_test_role();

    // Test workflow permissions
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Read, None));
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Create, None));
    assert!(!role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Update, None));
    assert!(!role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Delete, None));

    // Test entity permissions without path (should fail - path required)
    assert!(!role.has_permission(&ResourceNamespace::Entities, &PermissionType::Read, None));

    // Test entity permissions with matching path
    assert!(role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Read,
        Some("/projects")
    ));
    assert!(role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Read,
        Some("/projects/sub")
    ));
    assert!(role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Delete,
        Some("/projects")
    ));

    // Test entity permissions with non-matching path
    assert!(!role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Read,
        Some("/other")
    ));
    assert!(!role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Read,
        Some("/other/path")
    ));
}

#[test]
fn test_super_admin_has_all_permissions() {
    let mut role = Role::new("Super Admin".to_string());
    role.super_admin = true;

    // Super admin should have all permissions regardless of permissions array
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Read, None));
    assert!(role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Delete,
        Some("/any/path")
    ));
    assert!(role.has_permission(&ResourceNamespace::System, &PermissionType::Admin, None));
}

#[test]
fn test_path_constraint_matching() {
    let _role = Role::new("Test".to_string());

    // No constraint - all paths allowed
    assert!(Role::check_path_constraint(None, "/any/path"));

    // Exact match
    let constraints = serde_json::json!({"path": "/projects"});
    assert!(Role::check_path_constraint(Some(&constraints), "/projects"));
    assert!(!Role::check_path_constraint(Some(&constraints), "/project")); // Not exact

    // Prefix match
    assert!(Role::check_path_constraint(
        Some(&constraints),
        "/projects/sub"
    ));
    assert!(Role::check_path_constraint(
        Some(&constraints),
        "/projects/sub/deep"
    ));

    // Non-matching paths
    assert!(!Role::check_path_constraint(Some(&constraints), "/other"));
    assert!(!Role::check_path_constraint(
        Some(&constraints),
        "/projectx"
    )); // Prefix but not valid

    // Wildcard match
    let wildcard_constraints = serde_json::json!({"path": "/projects/*"});
    assert!(Role::check_path_constraint(
        Some(&wildcard_constraints),
        "/projects/sub"
    ));
    assert!(Role::check_path_constraint(
        Some(&wildcard_constraints),
        "/projects/sub/deep"
    ));
    assert!(!Role::check_path_constraint(
        Some(&wildcard_constraints),
        "/projects"
    ));
}

#[test]
fn test_get_permissions_as_strings() {
    let role = create_test_role();

    let perms = role.get_permissions_as_strings();
    assert!(perms.contains(&"workflows:read".to_string()));
    assert!(perms.contains(&"workflows:create".to_string()));
    assert!(perms.contains(&"entities:/projects:read".to_string()));
    assert!(perms.contains(&"entities:/projects:delete".to_string()));
    assert_eq!(perms.len(), 4);
}

#[test]
fn test_add_remove_permission() {
    let mut role = Role::new("Test".to_string());

    // Add permission
    let perm = Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    };

    assert!(role.add_permission(perm.clone()).is_ok());
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Read, None));

    // Try to add duplicate
    assert!(role.add_permission(perm).is_err());

    // Remove permission
    assert!(role
        .remove_permission(&ResourceNamespace::Workflows, &PermissionType::Read)
        .is_ok());
    assert!(!role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Read, None));
}

#[test]
fn test_system_role_cannot_be_modified() {
    let mut role = Role::new("System Role".to_string());
    role.is_system = true;

    let perm = Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Read,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    };

    assert!(role.add_permission(perm).is_err());
    assert!(role
        .remove_permission(&ResourceNamespace::Workflows, &PermissionType::Read)
        .is_err());
}

#[test]
fn test_admin_permission_grants_all_permissions_for_namespace() {
    let mut role = Role::new("Admin Role".to_string());

    // Add Admin permission for Workflows namespace
    role.add_permission(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    })
    .unwrap();

    // Admin should grant all permission types for Workflows namespace
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Read, None));
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Create, None));
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Update, None));
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Delete, None));
    assert!(role.has_permission(
        &ResourceNamespace::Workflows,
        &PermissionType::Publish,
        None
    ));
    assert!(role.has_permission(
        &ResourceNamespace::Workflows,
        &PermissionType::Execute,
        None
    ));
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Admin, None));

    // But Admin for Workflows should NOT grant permissions for other namespaces
    assert!(!role.has_permission(&ResourceNamespace::Entities, &PermissionType::Read, None));
    assert!(!role.has_permission(&ResourceNamespace::System, &PermissionType::Read, None));
}

#[test]
fn test_admin_permission_independent_per_namespace() {
    let mut role = Role::new("Multi Admin Role".to_string());

    // Add Admin permission for Workflows
    role.add_permission(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    })
    .unwrap();

    // Add Admin permission for Entities
    role.add_permission(Permission {
        resource_type: ResourceNamespace::Entities,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    })
    .unwrap();

    // Should have all permissions for Workflows
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Read, None));
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Delete, None));

    // Should have all permissions for Entities
    assert!(role.has_permission(&ResourceNamespace::Entities, &PermissionType::Read, None));
    assert!(role.has_permission(&ResourceNamespace::Entities, &PermissionType::Delete, None));

    // But should NOT have permissions for System (no Admin for System)
    assert!(!role.has_permission(&ResourceNamespace::System, &PermissionType::Read, None));
}

#[test]
fn test_admin_vs_super_admin_distinction() {
    let mut role = Role::new("Admin Role".to_string());

    // Add Admin permission for Workflows only
    role.add_permission(Permission {
        resource_type: ResourceNamespace::Workflows,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: None,
    })
    .unwrap();

    // Should have permissions for Workflows
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Read, None));

    // Should NOT have permissions for System
    assert!(!role.has_permission(&ResourceNamespace::System, &PermissionType::Read, None));

    // Now make it super_admin
    role.super_admin = true;

    // Should now have permissions for ALL namespaces
    assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Read, None));
    assert!(role.has_permission(&ResourceNamespace::System, &PermissionType::Read, None));
    assert!(role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Delete,
        Some("/any/path")
    ));
}

#[test]
fn test_execute_permission_only_for_workflows() {
    let mut role = Role::new("Test Role".to_string());

    // Execute permission for Workflows should succeed
    assert!(role
        .add_permission(Permission {
            resource_type: ResourceNamespace::Workflows,
            permission_type: PermissionType::Execute,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        })
        .is_ok());

    // Execute permission for Entities should fail
    assert!(role
        .add_permission(Permission {
            resource_type: ResourceNamespace::Entities,
            permission_type: PermissionType::Execute,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        })
        .is_err());

    // Execute permission for System should fail
    assert!(role
        .add_permission(Permission {
            resource_type: ResourceNamespace::System,
            permission_type: PermissionType::Execute,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        })
        .is_err());
}

#[test]
fn test_admin_permission_with_entities_path_constraint() {
    let mut role = Role::new("Admin Role".to_string());

    // Add Admin permission for Entities with path constraint
    role.add_permission(Permission {
        resource_type: ResourceNamespace::Entities,
        permission_type: PermissionType::Admin,
        access_level: AccessLevel::All,
        resource_uuids: vec![],
        constraints: Some(serde_json::json!({"path": "/projects"})),
    })
    .unwrap();

    // Should have all permissions for Entities under /projects path
    assert!(role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Read,
        Some("/projects")
    ));
    assert!(role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Delete,
        Some("/projects/sub")
    ));

    // Should NOT have permissions for Entities under other paths
    assert!(!role.has_permission(
        &ResourceNamespace::Entities,
        &PermissionType::Read,
        Some("/other")
    ));

    // Should NOT have permissions without path (when Admin has path constraint)
    assert!(!role.has_permission(&ResourceNamespace::Entities, &PermissionType::Read, None));
}
