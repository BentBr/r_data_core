#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use r_data_core_core::permissions::role::{
    AccessLevel, Permission, PermissionType, ResourceNamespace, Role,
};

/// Permission response DTO (for API serialization)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PermissionResponse {
    /// Resource type (as string for API compatibility)
    pub resource_type: String,
    /// Permission type
    pub permission_type: PermissionType,
    /// Access level
    pub access_level: AccessLevel,
    /// Resource UUIDs this permission applies to
    pub resource_uuids: Vec<Uuid>,
    /// Additional constraints
    pub constraints: Option<serde_json::Value>,
}

impl From<&Permission> for PermissionResponse {
    fn from(permission: &Permission) -> Self {
        Self {
            resource_type: permission.resource_type.as_str().to_string(),
            permission_type: permission.permission_type.clone(),
            access_level: permission.access_level.clone(),
            resource_uuids: permission.resource_uuids.clone(),
            constraints: permission.constraints.clone(),
        }
    }
}

impl TryFrom<PermissionResponse> for Permission {
    type Error = String;

    fn try_from(response: PermissionResponse) -> Result<Self, Self::Error> {
        // Convert PascalCase to snake_case (lowercase)
        // Handles both "EntityDefinitions" -> "entity_definitions" and "entity_definitions" -> "entity_definitions"
        let resource_type_str = pascal_to_snake_case(&response.resource_type);
        let resource_type = ResourceNamespace::try_from_str(&resource_type_str)
            .ok_or_else(|| format!("Invalid resource type: {}", response.resource_type))?;

        Ok(Self {
            resource_type,
            permission_type: response.permission_type,
            access_level: response.access_level,
            resource_uuids: response.resource_uuids,
            constraints: response.constraints,
        })
    }
}

/// Convert `PascalCase` to `snake_case`
/// Example: "`EntityDefinitions`" -> "`entity_definitions`", "`ApiKeys`" -> "`api_keys`"
pub(crate) fn pascal_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap_or(c));
    }
    result
}

/// Role response DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleResponse {
    /// UUID of the role
    pub uuid: Uuid,
    /// Name of the role
    pub name: String,
    /// Description of the role
    pub description: Option<String>,
    /// Whether this is a system role
    pub is_system: bool,
    /// Whether this role grants super admin privileges
    pub super_admin: bool,
    /// Direct permissions for this role
    pub permissions: Vec<PermissionResponse>,
    /// When the role was created
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When the role was last updated
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// UUID of the user who created the role
    pub created_by: Uuid,
    /// UUID of the user who last updated the role
    pub updated_by: Option<Uuid>,
    /// Whether the role is published
    pub published: bool,
    /// Version number
    pub version: i32,
}

impl From<&Role> for RoleResponse {
    fn from(role: &Role) -> Self {
        Self {
            uuid: role.base.uuid,
            name: role.name.clone(),
            description: role.description.clone(),
            is_system: role.is_system,
            super_admin: role.super_admin,
            permissions: role
                .permissions
                .iter()
                .map(PermissionResponse::from)
                .collect(),
            created_at: role.base.created_at,
            updated_at: role.base.updated_at,
            created_by: role.base.created_by,
            updated_by: role.base.updated_by,
            published: role.base.published,
            version: role.base.version,
        }
    }
}

/// Request to create a new role
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    /// Name of the role
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Whether this role grants super admin privileges
    pub super_admin: Option<bool>,
    /// Direct permissions for this role
    pub permissions: Vec<PermissionResponse>,
}

/// Request to update an existing role
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    /// Name of the role
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Whether this role grants super admin privileges
    pub super_admin: Option<bool>,
    /// Direct permissions for this role
    pub permissions: Vec<PermissionResponse>,
}

/// Request to assign roles to a user or API key
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AssignRolesRequest {
    /// UUIDs of roles to assign
    pub role_uuids: Vec<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use r_data_core_core::permissions::role::{AccessLevel, PermissionType};

    #[test]
    fn test_pascal_to_snake_case() {
        // Test PascalCase conversion
        assert_eq!(
            pascal_to_snake_case("EntityDefinitions"),
            "entity_definitions"
        );
        assert_eq!(pascal_to_snake_case("ApiKeys"), "api_keys");
        assert_eq!(pascal_to_snake_case("Roles"), "roles");

        // Test single word (no underscores needed)
        assert_eq!(pascal_to_snake_case("Workflows"), "workflows");
        assert_eq!(pascal_to_snake_case("Entities"), "entities");
        assert_eq!(pascal_to_snake_case("System"), "system");

        // Test already snake_case (should remain unchanged)
        assert_eq!(
            pascal_to_snake_case("entity_definitions"),
            "entity_definitions"
        );
        assert_eq!(pascal_to_snake_case("api_keys"), "api_keys");
        assert_eq!(pascal_to_snake_case("workflows"), "workflows");

        // Test lowercase (should remain unchanged)
        assert_eq!(pascal_to_snake_case("workflows"), "workflows");
        assert_eq!(pascal_to_snake_case("entities"), "entities");
    }

    #[test]
    fn test_permission_response_try_from_with_pascal_case() {
        // Test that PermissionResponse with PascalCase resource_type converts correctly
        let response = PermissionResponse {
            resource_type: "EntityDefinitions".to_string(),
            permission_type: PermissionType::Read,
            access_level: AccessLevel::All,
            resource_uuids: Vec::new(),
            constraints: None,
        };

        let permission = Permission::try_from(response);
        assert!(permission.is_ok());
        assert_eq!(
            permission.unwrap().resource_type,
            ResourceNamespace::EntityDefinitions
        );
    }

    #[test]
    fn test_permission_response_try_from_with_snake_case() {
        // Test that PermissionResponse with snake_case resource_type still works
        let response = PermissionResponse {
            resource_type: "entity_definitions".to_string(),
            permission_type: PermissionType::Read,
            access_level: AccessLevel::All,
            resource_uuids: Vec::new(),
            constraints: None,
        };

        let permission = Permission::try_from(response);
        assert!(permission.is_ok());
        assert_eq!(
            permission.unwrap().resource_type,
            ResourceNamespace::EntityDefinitions
        );
    }

    #[test]
    fn test_permission_response_try_from_all_resource_types() {
        let test_cases = vec![
            ("Workflows", ResourceNamespace::Workflows),
            ("workflows", ResourceNamespace::Workflows),
            ("Entities", ResourceNamespace::Entities),
            ("entities", ResourceNamespace::Entities),
            ("EntityDefinitions", ResourceNamespace::EntityDefinitions),
            ("entity_definitions", ResourceNamespace::EntityDefinitions),
            ("ApiKeys", ResourceNamespace::ApiKeys),
            ("api_keys", ResourceNamespace::ApiKeys),
            ("Roles", ResourceNamespace::Roles),
            ("roles", ResourceNamespace::Roles),
            ("System", ResourceNamespace::System),
            ("system", ResourceNamespace::System),
        ];

        for (resource_type_str, expected_namespace) in test_cases {
            let response = PermissionResponse {
                resource_type: resource_type_str.to_string(),
                permission_type: PermissionType::Read,
                access_level: AccessLevel::All,
                resource_uuids: Vec::new(),
                constraints: None,
            };

            let permission = Permission::try_from(response);
            assert!(
                permission.is_ok(),
                "Failed to convert resource_type: {resource_type_str}"
            );
            assert_eq!(
                permission.unwrap().resource_type,
                expected_namespace,
                "Resource type mismatch for: {resource_type_str}"
            );
        }
    }

    #[test]
    fn test_permission_response_try_from_invalid_resource_type() {
        let response = PermissionResponse {
            resource_type: "InvalidResourceType".to_string(),
            permission_type: PermissionType::Read,
            access_level: AccessLevel::All,
            resource_uuids: Vec::new(),
            constraints: None,
        };

        let permission = Permission::try_from(response);
        assert!(permission.is_err());
        assert!(permission.unwrap_err().contains("Invalid resource type"));
    }
}
