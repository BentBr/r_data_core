#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use serde_json::Value;

/// Service for authentication and authorization operations
pub struct AuthService;

impl AuthService {
    /// Create a new auth service
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Get user's allowed routes and permissions
    ///
    /// # Arguments
    /// * `is_super_admin` - Whether the user is a super admin
    /// * `permissions` - User permissions from JWT
    /// * `has_permission_fn` - Function to check if user has a specific permission
    ///
    /// # Returns
    /// JSON value containing user permissions and allowed routes
    #[must_use]
    pub fn get_user_permissions<F>(
        &self,
        is_super_admin: bool,
        permissions: &[String],
        has_permission_fn: F,
    ) -> Value
    where
        F: Fn(&ResourceNamespace, &PermissionType) -> bool,
    {
        // Map routes to required permissions
        let route_permissions: Vec<(&str, ResourceNamespace, PermissionType)> = vec![
            (
                "/dashboard",
                ResourceNamespace::DashboardStats,
                PermissionType::Read,
            ),
            (
                "/workflows",
                ResourceNamespace::Workflows,
                PermissionType::Read,
            ),
            (
                "/entity-definitions",
                ResourceNamespace::EntityDefinitions,
                PermissionType::Read,
            ),
            (
                "/entities",
                ResourceNamespace::Entities,
                PermissionType::Read,
            ),
            (
                "/api-keys",
                ResourceNamespace::ApiKeys,
                PermissionType::Read,
            ),
            (
                "/permissions",
                ResourceNamespace::Users,
                PermissionType::Read,
            ),
            ("/system", ResourceNamespace::System, PermissionType::Read),
        ];

        // Check which routes the user can access
        let allowed_routes: Vec<String> = route_permissions
            .iter()
            .filter_map(|(route, namespace, perm_type)| {
                if has_permission_fn(namespace, perm_type) {
                    Some(route.to_string())
                } else {
                    None
                }
            })
            .collect();

        // Build response
        serde_json::json!({
            "is_super_admin": is_super_admin,
            "permissions": permissions,
            "allowed_routes": allowed_routes,
        })
    }
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}
