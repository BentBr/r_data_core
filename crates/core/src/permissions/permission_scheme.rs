#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

use crate::domain::AbstractRDataEntity;
use crate::error::{Error, Result};

/// Permission types that can be granted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

    /// Custom permission
    Custom(String),
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
            Self::Custom(name) => write!(f, "Custom({name})"),
        }
    }
}

/// Access level for a permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceNamespace {
    /// Workflows namespace
    Workflows,

    /// Entities namespace (supports path-based permissions)
    Entities,

    /// Entity definitions namespace
    EntityDefinitions,

    /// API keys namespace
    ApiKeys,

    /// Permission schemes namespace
    PermissionSchemes,

    /// System settings namespace
    System,
}

impl ResourceNamespace {
    /// Get the string representation of the namespace
    ///
    /// # Returns
    /// String representation matching the namespace name
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Workflows => "workflows",
            Self::Entities => "entities",
            Self::EntityDefinitions => "entity_definitions",
            Self::ApiKeys => "api_keys",
            Self::PermissionSchemes => "permission_schemes",
            Self::System => "system",
        }
    }

    /// Convert a string to a ResourceNamespace
    ///
    /// # Arguments
    /// * `s` - String representation of the namespace
    ///
    /// # Returns
    /// ResourceNamespace enum variant, or None if invalid
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "workflows" => Some(Self::Workflows),
            "entities" => Some(Self::Entities),
            "entity_definitions" => Some(Self::EntityDefinitions),
            "api_keys" => Some(Self::ApiKeys),
            "permission_schemes" => Some(Self::PermissionSchemes),
            "system" => Some(Self::System),
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

/// Entity for defining a permission scheme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionScheme {
    /// Base entity properties
    pub base: AbstractRDataEntity,

    /// Scheme name
    pub name: String,

    /// Scheme description
    pub description: Option<String>,

    /// Whether this is a system scheme (cannot be modified)
    pub is_system: bool,

    /// Role-based permissions
    /// Key is role name (e.g., "MyRole"), value is list of permissions for that role
    pub role_permissions: HashMap<String, Vec<Permission>>,
}

impl PermissionScheme {
    /// Create a new permission scheme
    ///
    /// # Arguments
    /// * `name` - Name of the permission scheme
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            base: AbstractRDataEntity::new("/admin/permission-schemes".to_string()),
            name,
            description: None,
            is_system: false,
            role_permissions: HashMap::new(),
        }
    }

    /// Add a permission to a role
    ///
    /// # Arguments
    /// * `role` - Role name
    /// * `permission` - Permission to add
    ///
    /// # Errors
    /// Returns an error if the scheme is system or permission already exists
    pub fn add_permission(&mut self, role: &str, permission: Permission) -> Result<()> {
        if self.is_system {
            return Err(Error::Entity(
                "Cannot modify a system permission scheme".to_string(),
            ));
        }

        // Get or create the role permissions list
        let permissions = self.role_permissions.entry(role.to_string()).or_default();

        // Check if permission already exists
        if permissions.contains(&permission) {
            return Err(Error::Entity(format!(
                "Permission {}.{} already exists for role {role}",
                permission.resource_type.as_str(), permission.permission_type
            )));
        }

        // Add the permission
        permissions.push(permission);
        Ok(())
    }

    /// Remove a permission from a role
    ///
    /// # Arguments
    /// * `role` - Role name
    /// * `resource_type` - Resource namespace
    /// * `permission_type` - Permission type
    ///
    /// # Errors
    /// Returns an error if the scheme is system, role not found, or permission not found
    pub fn remove_permission(
        &mut self,
        role: &str,
        resource_type: &ResourceNamespace,
        permission_type: &PermissionType,
    ) -> Result<()> {
        if self.is_system {
            return Err(Error::Entity(
                "Cannot modify a system permission scheme".to_string(),
            ));
        }

        let permissions = self
            .role_permissions
            .get_mut(role)
            .ok_or_else(|| Error::Entity(format!("Role {role} not found")))?;

        permissions
            .iter()
            .position(|p| {
                p.resource_type == *resource_type && p.permission_type == *permission_type
            })
            .map_or_else(
                || {
                    Err(Error::Entity(format!(
                        "Permission {}.{} not found for role {role}",
                        resource_type.as_str(), permission_type
                    )))
                },
                |perm_idx| {
                    permissions.remove(perm_idx);
                    Ok(())
                },
            )
    }

    /// Check if a role has a specific permission
    ///
    /// # Arguments
    /// * `role` - Role name
    /// * `namespace` - Resource namespace
    /// * `permission_type` - Permission type
    /// * `path` - Optional path constraint (for entities namespace)
    ///
    /// # Returns
    /// `true` if the role has the permission, `false` otherwise
    ///
    /// # Examples
    /// ```
    /// // Check if role can read workflows
    /// scheme.has_permission("MyRole", &ResourceNamespace::Workflows, &PermissionType::Read, None);
    ///
    /// // Check if role can read entities under /projects path
    /// scheme.has_permission("MyRole", &ResourceNamespace::Entities, &PermissionType::Read, Some("/projects"));
    /// ```
    #[must_use]
    pub fn has_permission(
        &self,
        role: &str,
        namespace: &ResourceNamespace,
        permission_type: &PermissionType,
        path: Option<&str>,
    ) -> bool {
        self.role_permissions
            .get(role)
            .is_some_and(|permissions| {
                permissions.iter().any(|p| {
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
                            return self.check_path_constraint(&p.constraints, requested_path);
                        }
                    }

                    true
                })
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
    fn check_path_constraint(
        &self,
        constraints: &Option<serde_json::Value>,
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

        // Wildcard match: /projects/* matches /projects/sub
        if allowed_path.ends_with("/*") {
            let base = &allowed_path[..allowed_path.len() - 2];
            if requested_path.starts_with(base)
                && (requested_path.len() == base.len()
                    || requested_path.as_bytes()[base.len()] == b'/')
            {
                return true;
            }
        }

        false
    }

    /// Get all permissions for a role
    ///
    /// # Arguments
    /// * `role` - Role name
    ///
    /// # Returns
    /// Optional reference to the permissions vector for the role
    #[must_use]
    pub fn get_role_permissions(&self, role: &str) -> Option<&Vec<Permission>> {
        self.role_permissions.get(role)
    }

    /// Get all permissions for a role formatted as namespace strings
    ///
    /// # Arguments
    /// * `role` - Role name
    ///
    /// # Returns
    /// Vector of permission strings in format: `"{namespace}:{permission_type}"` or
    /// `"{namespace}:{path}:{permission_type}"` for entities with path constraints
    ///
    /// # Examples
    /// - `["workflows:read", "workflows:create"]`
    /// - `["entities:/projects:read", "entities:/projects:delete"]`
    #[must_use]
    pub fn get_permissions_as_strings(&self, role: &str) -> Vec<String> {
        let Some(permissions) = self.role_permissions.get(role) else {
            return Vec::new();
        };

        let mut result = Vec::new();
        for perm in permissions {
            let perm_str = format!("{}", perm.permission_type);
            let perm_str_lower = perm_str.to_lowercase();

            // For entities namespace, include path in permission string if present
            if matches!(perm.resource_type, ResourceNamespace::Entities) {
                if let Some(constraints) = &perm.constraints {
                    if let Some(path_value) = constraints.get("path") {
                        if let Some(path) = path_value.as_str() {
                            result.push(format!("{}:{}:{}", perm.resource_type.as_str(), path, perm_str_lower));
                            continue;
                        }
                    }
                }
            }

            // Standard format: namespace:permission
            result.push(format!("{}:{}", perm.resource_type.as_str(), perm_str_lower));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_scheme() -> PermissionScheme {
        let mut scheme = PermissionScheme::new("Test Scheme".to_string());
        let role = "MyRole".to_string();

        // Add workflow permissions
        scheme
            .add_permission(
                &role,
                Permission {
                    resource_type: ResourceNamespace::Workflows,
                    permission_type: PermissionType::Read,
                    access_level: AccessLevel::All,
                    resource_uuids: vec![],
                    constraints: None,
                },
            )
            .unwrap();

        scheme
            .add_permission(
                &role,
                Permission {
                    resource_type: ResourceNamespace::Workflows,
                    permission_type: PermissionType::Create,
                    access_level: AccessLevel::All,
                    resource_uuids: vec![],
                    constraints: None,
                },
            )
            .unwrap();

        // Add entity permissions with path constraint
        scheme
            .add_permission(
                &role,
                Permission {
                    resource_type: ResourceNamespace::Entities,
                    permission_type: PermissionType::Read,
                    access_level: AccessLevel::All,
                    resource_uuids: vec![],
                    constraints: Some(serde_json::json!({"path": "/projects"})),
                },
            )
            .unwrap();

        scheme
            .add_permission(
                &role,
                Permission {
                    resource_type: ResourceNamespace::Entities,
                    permission_type: PermissionType::Delete,
                    access_level: AccessLevel::All,
                    resource_uuids: vec![],
                    constraints: Some(serde_json::json!({"path": "/projects"})),
                },
            )
            .unwrap();

        scheme
    }

    #[test]
    fn test_has_permission_namespace() {
        let scheme = create_test_scheme();

        // Test workflow permissions
        assert!(scheme.has_permission("MyRole", &ResourceNamespace::Workflows, &PermissionType::Read, None));
        assert!(scheme.has_permission("MyRole", &ResourceNamespace::Workflows, &PermissionType::Create, None));
        assert!(!scheme.has_permission("MyRole", &ResourceNamespace::Workflows, &PermissionType::Update, None));
        assert!(!scheme.has_permission("MyRole", &ResourceNamespace::Workflows, &PermissionType::Delete, None));

        // Test entity permissions without path (should fail - path required)
        assert!(!scheme.has_permission("MyRole", &ResourceNamespace::Entities, &PermissionType::Read, None));

        // Test entity permissions with matching path
        assert!(scheme.has_permission("MyRole", &ResourceNamespace::Entities, &PermissionType::Read, Some("/projects")));
        assert!(scheme.has_permission("MyRole", &ResourceNamespace::Entities, &PermissionType::Read, Some("/projects/sub")));
        assert!(scheme.has_permission("MyRole", &ResourceNamespace::Entities, &PermissionType::Delete, Some("/projects")));

        // Test entity permissions with non-matching path
        assert!(!scheme.has_permission("MyRole", &ResourceNamespace::Entities, &PermissionType::Read, Some("/other")));
        assert!(!scheme.has_permission("MyRole", &ResourceNamespace::Entities, &PermissionType::Read, Some("/other/path")));
    }

    #[test]
    fn test_path_constraint_matching() {
        let scheme = PermissionScheme::new("Test".to_string());

        // No constraint - all paths allowed
        assert!(scheme.check_path_constraint(&None, "/any/path"));

        // Exact match
        let constraints = Some(serde_json::json!({"path": "/projects"}));
        assert!(scheme.check_path_constraint(&constraints, "/projects"));
        assert!(!scheme.check_path_constraint(&constraints, "/project")); // Not exact

        // Prefix match
        assert!(scheme.check_path_constraint(&constraints, "/projects/sub"));
        assert!(scheme.check_path_constraint(&constraints, "/projects/sub/deep"));

        // Non-matching paths
        assert!(!scheme.check_path_constraint(&constraints, "/other"));
        assert!(!scheme.check_path_constraint(&constraints, "/projectx")); // Prefix but not valid

        // Wildcard match
        let wildcard_constraints = Some(serde_json::json!({"path": "/projects/*"}));
        assert!(scheme.check_path_constraint(&wildcard_constraints, "/projects/sub"));
        assert!(scheme.check_path_constraint(&wildcard_constraints, "/projects/sub/deep"));
        assert!(!scheme.check_path_constraint(&wildcard_constraints, "/projects")); // Exact match still works
    }

    #[test]
    fn test_get_permissions_as_strings() {
        let scheme = create_test_scheme();

        let perms = scheme.get_permissions_as_strings("MyRole");
        assert!(perms.contains(&"workflows:read".to_string()));
        assert!(perms.contains(&"workflows:create".to_string()));
        assert!(perms.contains(&"entities:/projects:read".to_string()));
        assert!(perms.contains(&"entities:/projects:delete".to_string()));
        assert_eq!(perms.len(), 4);
    }

    #[test]
    fn test_add_remove_permission() {
        let mut scheme = PermissionScheme::new("Test".to_string());
        let role = "TestRole";

        // Add permission
        let perm = Permission {
            resource_type: ResourceNamespace::Workflows,
            permission_type: PermissionType::Read,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        };

        assert!(scheme.add_permission(role, perm.clone()).is_ok());
        assert!(scheme.has_permission(role, &ResourceNamespace::Workflows, &PermissionType::Read, None));

        // Try to add duplicate
        assert!(scheme.add_permission(role, perm).is_err());

        // Remove permission
        assert!(scheme
            .remove_permission(role, &ResourceNamespace::Workflows, &PermissionType::Read)
            .is_ok());
        assert!(!scheme.has_permission(role, &ResourceNamespace::Workflows, &PermissionType::Read, None));
    }
}
