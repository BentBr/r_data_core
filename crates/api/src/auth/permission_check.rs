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
/// `SuperAdmin` always has all permissions
#[must_use]
pub fn has_permission(
    claims: &AuthUserClaims,
    namespace: &ResourceNamespace,
    permission_type: &PermissionType,
    path: Option<&str>,
) -> bool {
    // SuperAdmin role or super_admin flag always has all permissions
    if claims.is_super_admin || claims.role == "SuperAdmin" {
        debug!(
            "Permission check: {} user '{}' (super_admin: {}) has all permissions for {}:{}",
            if claims.is_super_admin {
                "super_admin"
            } else {
                "SuperAdmin"
            },
            claims.name,
            claims.is_super_admin,
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
                && path.map_or(false, |req_path| {
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

/// Check if an API key has permission to perform an action
///
/// This function loads permission schemes for the API key, merges permissions,
/// and checks if the required permission is present.
///
/// # Arguments
/// * `api_key_uuid` - API key UUID
/// * `namespace` - Resource namespace
/// * `permission_type` - Permission type (read, create, update, delete, etc.)
/// * `path` - Optional path constraint (for entities namespace)
/// * `permission_scheme_service` - Permission scheme service for loading schemes
/// * `api_key_repo` - API key repository for loading scheme assignments
///
/// # Returns
/// `true` if the API key has permission, `false` otherwise
///
/// # Note
/// API keys don't have roles, so permissions from all roles in assigned schemes are merged
///
/// # Errors
/// Returns an error if the API key cannot be found or if there's a database error
pub async fn has_permission_for_api_key(
    api_key_uuid: uuid::Uuid,
    namespace: &ResourceNamespace,
    permission_type: &PermissionType,
    path: Option<&str>,
    permission_scheme_service: &r_data_core_services::PermissionSchemeService,
    api_key_repo: &r_data_core_persistence::ApiKeyRepository,
) -> r_data_core_core::error::Result<bool> {
    use std::collections::HashSet;

    // Load all permission schemes for the API key
    let schemes = permission_scheme_service
        .get_schemes_for_api_key(api_key_uuid, api_key_repo)
        .await?;

    if schemes.is_empty() {
        debug!(
            "Permission check: API key {api_key_uuid} has no permission schemes assigned"
        );
        return Ok(false);
    }

    // Merge permissions from all schemes
    // Since API keys don't have roles, we need to collect permissions from all roles
    let mut permission_set = HashSet::new();
    for scheme in &schemes {
        // Get permissions from all roles in the scheme
        for (_role, permissions) in &scheme.role_permissions {
            for permission in permissions {
                // Only check permissions that match the requested namespace and type
                if permission.resource_type == *namespace
                    && permission.permission_type == *permission_type
                {
                    // For entities namespace, check path constraints
                    if matches!(namespace, ResourceNamespace::Entities) {
                        if let Some(requested_path) = path {
                            // Check if path constraint allows this path
                            let allowed = permission.constraints
                                .as_ref()
                                .and_then(|c| c.get("path"))
                                .and_then(|v| v.as_str())
                                .map_or(true, |allowed_path| {
                                    // Exact match or prefix match
                                    requested_path == allowed_path
                                        || requested_path.starts_with(allowed_path)
                                });

                            if allowed {
                                permission_set.insert(format!(
                                    "{}:{requested_path}:{}",
                                    namespace.as_str(),
                                    format!("{permission_type}").to_lowercase()
                                ));
                            }
                        } else {
                            // No path requested, check if permission has no path constraint
                            let has_path_constraint = permission
                                .constraints
                                .as_ref()
                                .and_then(|c| c.get("path"))
                                .is_some();
                            if !has_path_constraint {
                                let perm_type_str = format!("{permission_type}").to_lowercase();
                                permission_set.insert(format!(
                                    "{}:{perm_type_str}",
                                    namespace.as_str()
                                ));
                            }
                        }
                    } else {
                        // Non-entities namespace, no path checking needed
                        let perm_type_str = format!("{permission_type}").to_lowercase();
                        permission_set.insert(format!(
                            "{}:{perm_type_str}",
                            namespace.as_str()
                        ));
                    }
                }
            }
        }
    }

    // Build permission string to check
    let perm_str = format!("{permission_type}").to_lowercase();
    let namespace_str = namespace.as_str();
    let permission_string = path.map_or_else(
        || format!("{namespace_str}:{perm_str}"),
        |p| format!("{namespace_str}:{p}:{perm_str}"),
    );

    // Check if merged permissions include the required permission
    let has_perm = permission_set.contains(&permission_string)
        || (matches!(namespace, ResourceNamespace::Entities)
            && path.is_some()
            && permission_set.iter().any(|p| {
                p.starts_with(&format!("{namespace_str}:"))
                    && p.ends_with(&format!(":{perm_str}"))
                    && if let Some(req_path) = path {
                        if let Some(perm_path) = p.strip_prefix(&format!("{namespace_str}:"))
                        {
                            if let Some(perm_path) =
                                perm_path.strip_suffix(&format!(":{perm_str}"))
                            {
                                return req_path.starts_with(perm_path);
                            }
                        }
                        false
                    } else {
                        false
                    }
            }));

    debug!(
        "Permission check: API key {} {} permission for {}:{} (path: {:?})",
        api_key_uuid,
        if has_perm { "has" } else { "does not have" },
        namespace.as_str(),
        permission_type,
        path
    );

    Ok(has_perm)
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
