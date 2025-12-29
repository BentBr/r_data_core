#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::AbstractRDataEntity;
use crate::error::{Error, Result};

/// Permission types that can be granted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub enum PermissionType {
    /// Read data
    Read,

    /// Create new data
    Create,

    /// Update existing data
    Update,

    /// Delete data
    Delete,

    /// Publish data
    Publish,

    /// Administrative action
    Admin,

    /// Execute a workflow
    Execute,
}

impl Display for PermissionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read => write!(f, "Read"),
            Self::Create => write!(f, "Create"),
            Self::Update => write!(f, "Update"),
            Self::Delete => write!(f, "Delete"),
            Self::Publish => write!(f, "Publish"),
            Self::Admin => write!(f, "Admin"),
            Self::Execute => write!(f, "Execute"),
        }
    }
}

/// Access level for a permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub enum AccessLevel {
    /// No access granted
    None,

    /// Access to own resources only
    Own,

    /// Access to resources in same group
    Group,

    /// Access to all resources
    All,
}

/// Resource namespace for permissions
///
/// Each namespace represents a different resource type that can have permissions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceNamespace {
    /// Workflows namespace
    Workflows,

    /// Entities namespace (supports path-based permissions)
    Entities,

    /// Entity definitions namespace
    EntityDefinitions,

    /// API keys namespace
    ApiKeys,

    /// Roles namespace
    Roles,

    /// System settings namespace
    System,

    /// Dashboard statistics namespace
    DashboardStats,
}

impl ResourceNamespace {
    /// Get the string representation of the namespace
    ///
    /// # Returns
    /// String representation matching the namespace name
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Workflows => "workflows",
            Self::Entities => "entities",
            Self::EntityDefinitions => "entity_definitions",
            Self::ApiKeys => "api_keys",
            Self::Roles => "roles",
            Self::System => "system",
            Self::DashboardStats => "dashboard_stats",
        }
    }

    /// Convert a string to a `ResourceNamespace`
    ///
    /// # Arguments
    /// * `s` - String representation of the namespace
    ///
    /// # Returns
    /// `ResourceNamespace` enum variant, or None if invalid
    #[must_use]
    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "workflows" => Some(Self::Workflows),
            "entities" => Some(Self::Entities),
            "entity_definitions" => Some(Self::EntityDefinitions),
            "api_keys" => Some(Self::ApiKeys),
            "roles" => Some(Self::Roles),
            "system" => Some(Self::System),
            "dashboard_stats" => Some(Self::DashboardStats),
            _ => None,
        }
    }
}

impl Display for ResourceNamespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A permission definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Permission {
    /// Resource namespace this permission applies to
    pub resource_type: ResourceNamespace,

    /// Permission type
    pub permission_type: PermissionType,

    /// Access level granted
    pub access_level: AccessLevel,

    /// Resource UUIDs this permission applies to (if empty, applies to all resources of type)
    pub resource_uuids: Vec<Uuid>,

    /// Additional constraints on this permission
    /// For entities namespace, can contain: {"path": "/projects"} for path-based permissions
    pub constraints: Option<serde_json::Value>,
}

/// Entity for defining a role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Base entity properties
    pub base: AbstractRDataEntity,

    /// Role name
    pub name: String,

    /// Role description
    pub description: Option<String>,

    /// Whether this is a system role (cannot be modified)
    pub is_system: bool,

    /// Whether this role grants super admin privileges
    /// Users with a `super_admin` role have all permissions regardless of `permissions`
    pub super_admin: bool,

    /// Direct permissions for this role
    pub permissions: Vec<Permission>,
}

impl Role {
    /// Create a new role
    ///
    /// # Arguments
    /// * `name` - Name of the role
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            base: AbstractRDataEntity::new("/admin/roles".to_string()),
            name,
            description: None,
            is_system: false,
            super_admin: false,
            permissions: Vec::new(),
        }
    }

    /// Add a permission to this role
    ///
    /// # Arguments
    /// * `permission` - Permission to add
    ///
    /// # Errors
    /// Returns an error if the role is system or permission already exists
    /// Returns an error if Execute permission is used with non-Workflows namespace
    pub fn add_permission(&mut self, permission: Permission) -> Result<()> {
        if self.is_system {
            return Err(Error::Entity("Cannot modify a system role".to_string()));
        }

        // Validate Execute permission can only be used with Workflows namespace
        if matches!(permission.permission_type, PermissionType::Execute)
            && !matches!(permission.resource_type, ResourceNamespace::Workflows)
        {
            return Err(Error::Entity(
                "Execute permission can only be used with Workflows namespace".to_string(),
            ));
        }

        // Check if permission already exists
        if self.permissions.contains(&permission) {
            return Err(Error::Entity(format!(
                "Permission {}.{} already exists",
                permission.resource_type.as_str(),
                permission.permission_type
            )));
        }

        // Add the permission
        self.permissions.push(permission);
        Ok(())
    }

    /// Remove a permission from this role
    ///
    /// # Arguments
    /// * `resource_type` - Resource namespace
    /// * `permission_type` - Permission type
    ///
    /// # Errors
    /// Returns an error if the role is system or permission not found
    pub fn remove_permission(
        &mut self,
        resource_type: &ResourceNamespace,
        permission_type: &PermissionType,
    ) -> Result<()> {
        if self.is_system {
            return Err(Error::Entity("Cannot modify a system role".to_string()));
        }

        let pos = self
            .permissions
            .iter()
            .position(|p| {
                p.resource_type == *resource_type && p.permission_type == *permission_type
            })
            .ok_or_else(|| {
                Error::Entity(format!(
                    "Permission {}.{} not found",
                    resource_type.as_str(),
                    permission_type
                ))
            })?;

        self.permissions.remove(pos);
        Ok(())
    }

    /// Check if this role has a specific permission
    ///
    /// # Arguments
    /// * `namespace` - Resource namespace
    /// * `permission_type` - Permission type
    /// * `path` - Optional path constraint (for entities namespace)
    ///
    /// # Returns
    /// `true` if the role has the permission, `false` otherwise
    ///
    /// # Note
    /// If `super_admin` is true, this always returns `true`.
    ///
    /// # Examples
    /// ```
    /// use r_data_core_core::permissions::role::{Role, ResourceNamespace, PermissionType, Permission, AccessLevel};
    ///
    /// let mut role = Role::new("Test".to_string());
    /// role.add_permission(Permission {
    ///     resource_type: ResourceNamespace::Workflows,
    ///     permission_type: PermissionType::Read,
    ///     access_level: AccessLevel::All,
    ///     resource_uuids: vec![],
    ///     constraints: None,
    /// }).unwrap();
    ///
    /// // Check if role can read workflows
    /// assert!(role.has_permission(&ResourceNamespace::Workflows, &PermissionType::Read, None));
    ///
    /// // Check if role can read entities under /projects path
    /// role.add_permission(Permission {
    ///     resource_type: ResourceNamespace::Entities,
    ///     permission_type: PermissionType::Read,
    ///     access_level: AccessLevel::All,
    ///     resource_uuids: vec![],
    ///     constraints: Some(serde_json::json!({"path": "/projects"})),
    /// }).unwrap();
    /// assert!(role.has_permission(&ResourceNamespace::Entities, &PermissionType::Read, Some("/projects")));
    /// ```
    #[must_use]
    pub fn has_permission(
        &self,
        namespace: &ResourceNamespace,
        permission_type: &PermissionType,
        path: Option<&str>,
    ) -> bool {
        // Global admin: Super admin roles have all permissions for all namespaces
        if self.super_admin {
            return true;
        }

        // Resource-level admin: Check if role has Admin permission for this namespace
        // Admin permission grants all permission types for the namespace
        let has_admin_for_namespace = self.permissions.iter().any(|p| {
            p.resource_type == *namespace && matches!(p.permission_type, PermissionType::Admin)
        });

        if has_admin_for_namespace {
            // For entities namespace with Admin, still need to check path constraints if provided
            if matches!(namespace, ResourceNamespace::Entities) {
                if let Some(requested_path) = path {
                    // Check if any Admin permission for this namespace allows the path
                    return self.permissions.iter().any(|p| {
                        p.resource_type == *namespace
                            && matches!(p.permission_type, PermissionType::Admin)
                            && Self::check_path_constraint(p.constraints.as_ref(), requested_path)
                    });
                }
                // If no path provided but Admin permission has path constraint, deny access
                // (Admin with path constraint only grants access when path matches)
                if self.permissions.iter().any(|p| {
                    p.resource_type == *namespace
                        && matches!(p.permission_type, PermissionType::Admin)
                        && p.constraints.is_some()
                }) {
                    return false;
                }
            }
            return true;
        }

        // Exact permission match
        self.permissions.iter().any(|p| {
            // Check namespace matches
            if p.resource_type != *namespace {
                return false;
            }

            // Check permission type matches
            if p.permission_type != *permission_type {
                return false;
            }

            // For entities namespace, check path constraints if provided
            if matches!(namespace, ResourceNamespace::Entities) {
                if let Some(requested_path) = path {
                    return Self::check_path_constraint(p.constraints.as_ref(), requested_path);
                }
                // If no path provided but permission has path constraint, deny access
                if p.constraints.is_some() {
                    return false;
                }
            }

            true
        })
    }

    /// Check if a path constraint allows access to the requested path
    ///
    /// # Arguments
    /// * `constraints` - Permission constraints JSON
    /// * `requested_path` - Path being accessed
    ///
    /// # Returns
    /// `true` if the path is allowed, `false` otherwise
    ///
    /// Path matching rules:
    /// - If no path constraint, allow all paths
    /// - Exact match: `/projects` matches only `/projects`
    /// - Prefix match: `/projects` matches `/projects/sub` and `/projects/sub/deep`
    /// - Wildcard: `/projects/*` explicitly allows sub-paths
    #[must_use]
    pub fn check_path_constraint(
        constraints: Option<&serde_json::Value>,
        requested_path: &str,
    ) -> bool {
        let Some(constraints) = constraints else {
            // No path constraint means all paths allowed
            return true;
        };

        let Some(path_value) = constraints.get("path") else {
            // No path in constraints means all paths allowed
            return true;
        };

        let Some(allowed_path) = path_value.as_str() else {
            return false;
        };

        // Exact match
        if allowed_path == requested_path {
            return true;
        }

        // Prefix match: /projects matches /projects/sub
        if requested_path.starts_with(allowed_path) {
            // Ensure it's a proper path segment (not just prefix)
            if requested_path.len() == allowed_path.len()
                || requested_path.as_bytes()[allowed_path.len()] == b'/'
            {
                return true;
            }
        }

        // Wildcard match: /projects/* matches /projects/sub but NOT /projects itself
        if let Some(base) = allowed_path.strip_suffix("/*") {
            if requested_path.starts_with(base)
                && requested_path.len() > base.len()
                && requested_path.as_bytes()[base.len()] == b'/'
            {
                return true;
            }
        }

        false
    }

    /// Get all permissions formatted as namespace strings
    ///
    /// # Returns
    /// Vector of permission strings in format: `"{namespace}:{permission_type}"` or
    /// `"{namespace}:{path}:{permission_type}"` for entities with path constraints
    ///
    /// # Examples
    /// - `["workflows:read", "workflows:create"]`
    /// - `["entities:/projects:read", "entities:/projects:delete"]`
    #[must_use]
    pub fn get_permissions_as_strings(&self) -> Vec<String> {
        let mut result = Vec::new();
        for perm in &self.permissions {
            let perm_str = format!("{}", perm.permission_type);
            let perm_str_lower = perm_str.to_lowercase();

            // For entities namespace, include path in permission string if present
            if matches!(perm.resource_type, ResourceNamespace::Entities) {
                if let Some(constraints) = &perm.constraints {
                    if let Some(path_value) = constraints.get("path") {
                        if let Some(path) = path_value.as_str() {
                            result.push(format!(
                                "{}:{}:{}",
                                perm.resource_type.as_str(),
                                path,
                                perm_str_lower
                            ));
                            continue;
                        }
                    }
                }
            }

            // Standard format: namespace:permission
            result.push(format!(
                "{}:{}",
                perm.resource_type.as_str(),
                perm_str_lower
            ));
        }

        result
    }
}

#[cfg(test)]
mod tests {
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
}
