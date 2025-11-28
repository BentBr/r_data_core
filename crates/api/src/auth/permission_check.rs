#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::jwt::AuthUserClaims;
use log::debug;
use r_data_core_core::permissions::permission_scheme::{PermissionType, ResourceNamespace};

/// Check if a user has permission to perform an action
///
/// # Arguments
/// * `claims` - JWT claims containing user role and permissions
/// * `namespace` - Resource namespace
/// * `permission_type` - Permission type (read, create, update, delete, etc.)
/// * `path` - Optional path constraint (for entities namespace)
///
/// # Returns
/// `true` if the user has permission, `false` otherwise
///
/// # Notes
/// SuperAdmin always has all permissions
#[must_use]
pub fn has_permission(
    claims: &AuthUserClaims,
    namespace: &ResourceNamespace,
    permission_type: &PermissionType,
    path: Option<&str>,
) -> bool {
    // SuperAdmin always has all permissions
    if claims.role == "SuperAdmin" {
        debug!(
            "Permission check: SuperAdmin user '{}' has all permissions for {}:{}",
            claims.name,
            namespace.as_str(),
            permission_type
        );
        return true;
    }

    // Build permission string to check
    let perm_str = format!("{}", permission_type).to_lowercase();
    let permission_string = if let Some(p) = path {
        format!("{}:{}:{}", namespace.as_str(), p, perm_str)
    } else {
        format!("{}:{}", namespace.as_str(), perm_str)
    };

    // Check if user has the permission
    let has_perm = claims.permissions.iter().any(|p| {
        // Exact match
        p == &permission_string
            // Or for entities with path, check if permission allows the path
            || (matches!(namespace, ResourceNamespace::Entities)
                && p.starts_with(&format!("{}:", namespace.as_str()))
                && p.ends_with(&format!(":{}", perm_str))
                && path.map_or(false, |req_path| {
                    // Extract path from permission string (format: "entities:/path:read")
                    if let Some(perm_path) = p.strip_prefix(&format!("{}:", namespace.as_str())) {
                        if let Some(perm_path) = perm_path.strip_suffix(&format!(":{}", perm_str)) {
                            // Check if requested path starts with permission path
                            return req_path.starts_with(perm_path);
                        }
                    }
                    false
                }))
    });

    debug!(
        "Permission check: user '{}' (role: {}) {} permission for {}:{} (path: {:?})",
        claims.name,
        claims.role,
        if has_perm { "has" } else { "does not have" },
        namespace.as_str(),
        permission_type,
        path
    );

    has_perm
}

/// Check if a user has permission to perform an action and log the result
///
/// This is a convenience wrapper around `has_permission` that also logs
/// the action being performed.
///
/// # Arguments
/// * `claims` - JWT claims containing user role and permissions
/// * `namespace` - Resource namespace
/// * `permission_type` - Permission type (read, create, update, delete, etc.)
/// * `path` - Optional path constraint (for entities namespace)
/// * `action` - Description of the action being performed (for logging)
///
/// # Returns
/// `true` if the user has permission, `false` otherwise
#[must_use]
pub fn check_permission_with_log(
    claims: &AuthUserClaims,
    namespace: &ResourceNamespace,
    permission_type: &PermissionType,
    path: Option<&str>,
    action: &str,
) -> bool {
    let has_perm = has_permission(claims, namespace, permission_type, path);

    debug!(
        "Action: {} | User: {} (role: {}) | Permission: {}:{} | Path: {:?} | Allowed: {}",
        action,
        claims.name,
        claims.role,
        namespace.as_str(),
        permission_type,
        path,
        has_perm
    );

    has_perm
}
