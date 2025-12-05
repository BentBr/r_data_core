#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::jwt::AuthUserClaims;
use log::debug;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};

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
/// `SuperAdmin` role or `super_admin` flag always has all permissions
#[must_use]
pub fn has_permission(
    claims: &AuthUserClaims,
    namespace: &ResourceNamespace,
    permission_type: &PermissionType,
    path: Option<&str>,
) -> bool {
    // Super admin flag always has all permissions
    if claims.is_super_admin {
        debug!(
            "Permission check: super_admin user '{}' has all permissions for {}:{}",
            claims.name,
            namespace.as_str(),
            permission_type
        );
        return true;
    }

    // Build permission string to check
    let perm_str = format!("{permission_type}").to_lowercase();
    let namespace_str = namespace.as_str();
    let permission_string = path.map_or_else(
        || format!("{namespace_str}:{perm_str}"),
        |p| format!("{namespace_str}:{p}:{perm_str}"),
    );

    // Check if user has the permission
    let has_perm = claims.permissions.iter().any(|p| {
        // Exact match
        p == &permission_string
            // Or for entities with path, check if permission allows the path
            || (matches!(namespace, ResourceNamespace::Entities)
                && p.starts_with(&format!("{namespace_str}:"))
                && p.ends_with(&format!(":{perm_str}"))
                && path.is_some_and(|req_path| {
                    // Extract path from permission string (format: "entities:/path:read")
                    let namespace_str = namespace.as_str();
                    if let Some(perm_path) = p.strip_prefix(&format!("{namespace_str}:")) {
                        if let Some(perm_path) = perm_path.strip_suffix(&format!(":{perm_str}")) {
                            // Check if requested path starts with permission path
                            return req_path.starts_with(perm_path);
                        }
                    }
                    false
                }))
    });

    debug!(
        "Permission check: user '{}' (super_admin: {}) {} permission for {}:{} (path: {:?})",
        claims.name,
        claims.is_super_admin,
        if has_perm { "has" } else { "does not have" },
        namespace.as_str(),
        permission_type,
        path
    );

    has_perm
}

/// Check if an API key has permission to perform an action
///
/// This function loads roles for the API key, merges permissions,
/// and checks if the required permission is present.
///
/// # Arguments
/// * `api_key_uuid` - API key UUID
/// * `namespace` - Resource namespace
/// * `permission_type` - Permission type (read, create, update, delete, etc.)
/// * `path` - Optional path constraint (for entities namespace)
/// * `role_service` - Role service for loading roles
/// * `api_key_repo` - API key repository for loading role assignments
///
/// # Returns
/// `true` if the API key has permission, `false` otherwise
///
/// # Note
/// API keys have roles, and permissions from all assigned roles are merged
///
/// # Errors
/// Returns an error if the API key cannot be found or if there's a database error
pub async fn has_permission_for_api_key(
    api_key_uuid: uuid::Uuid,
    namespace: &ResourceNamespace,
    permission_type: &PermissionType,
    path: Option<&str>,
    role_service: &r_data_core_services::RoleService,
    api_key_repo: &r_data_core_persistence::ApiKeyRepository,
) -> r_data_core_core::error::Result<bool> {
    // Load all roles for the API key
    let roles = role_service
        .get_roles_for_api_key(api_key_uuid, api_key_repo)
        .await?;

    if roles.is_empty() {
        debug!("Permission check: API key {api_key_uuid} has no roles assigned");
        return Ok(false);
    }

    // Check if any role is super_admin
    if roles.iter().any(|role| role.super_admin) {
        return Ok(true);
    }

    // Check permissions from all roles
    for role in &roles {
        // Check if role has the permission
        if role.has_permission(namespace, permission_type, path) {
            return Ok(true);
        }
    }

    Ok(false)
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
        "Action: {} | User: {} (super_admin: {}) | Permission: {}:{} | Path: {:?} | Allowed: {}",
        action,
        claims.name,
        claims.is_super_admin,
        namespace.as_str(),
        permission_type,
        path,
        has_perm
    );

    has_perm
}
