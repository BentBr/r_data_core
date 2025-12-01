#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use r_data_core_core::permissions::permission_scheme::{
    AccessLevel, Permission, PermissionScheme, PermissionType, ResourceNamespace,
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
        // Convert to lowercase for matching (API sends capitalized, but from_str expects lowercase)
        let resource_type_str = response.resource_type.to_lowercase();
        let resource_type = ResourceNamespace::from_str(&resource_type_str)
            .ok_or_else(|| format!("Invalid resource type: {}", response.resource_type))?;

        Ok(Permission {
            resource_type,
            permission_type: response.permission_type,
            access_level: response.access_level,
            resource_uuids: response.resource_uuids,
            constraints: response.constraints,
        })
    }
}

/// Permission scheme response DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PermissionSchemeResponse {
    /// UUID of the scheme
    pub uuid: Uuid,
    /// Name of the scheme
    pub name: String,
    /// Description of the scheme
    pub description: Option<String>,
    /// Whether this is a system scheme
    pub is_system: bool,
    /// Role-based permissions
    pub role_permissions: std::collections::HashMap<String, Vec<PermissionResponse>>,
    /// When the scheme was created
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// When the scheme was last updated
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// UUID of the user who created the scheme
    pub created_by: Uuid,
    /// UUID of the user who last updated the scheme
    pub updated_by: Option<Uuid>,
    /// Whether the scheme is published
    pub published: bool,
    /// Version number
    pub version: i32,
}

impl From<&PermissionScheme> for PermissionSchemeResponse {
    fn from(scheme: &PermissionScheme) -> Self {
        let mut role_permissions = std::collections::HashMap::new();
        for (role, permissions) in &scheme.role_permissions {
            role_permissions.insert(
                role.clone(),
                permissions.iter().map(PermissionResponse::from).collect(),
            );
        }

        Self {
            uuid: scheme.base.uuid,
            name: scheme.name.clone(),
            description: scheme.description.clone(),
            is_system: scheme.is_system,
            role_permissions,
            created_at: scheme.base.created_at,
            updated_at: scheme.base.updated_at,
            created_by: scheme.base.created_by,
            updated_by: scheme.base.updated_by,
            published: scheme.base.published,
            version: scheme.base.version,
        }
    }
}

/// Request to create a new permission scheme
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePermissionSchemeRequest {
    /// Name of the scheme
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Role-based permissions
    pub role_permissions: std::collections::HashMap<String, Vec<PermissionResponse>>,
}

/// Request to update an existing permission scheme
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdatePermissionSchemeRequest {
    /// Name of the scheme
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Role-based permissions
    pub role_permissions: std::collections::HashMap<String, Vec<PermissionResponse>>,
}

/// Request to assign permission schemes to a user or API key
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AssignSchemesRequest {
    /// UUIDs of permission schemes to assign
    pub scheme_uuids: Vec<Uuid>,
}
