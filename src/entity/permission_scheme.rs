use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, Row};
use std::collections::HashMap;
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;

use super::AbstractRDataEntity;
use crate::error::{Error, Result};

/// Permission types that can be granted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
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

/// Access level for a permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
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

/// A permission definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct Permission {
    /// Resource type this permission applies to
    pub resource_type: String,

    /// Permission type
    pub permission_type: PermissionType,

    /// Access level granted
    pub access_level: AccessLevel,

    /// Resource UUIDs this permission applies to (if empty, applies to all resources of type)
    pub resource_uuids: Vec<Uuid>,

    /// Additional constraints on this permission
    pub constraints: Option<serde_json::Value>,
}

/// Entity for defining a permission scheme
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
    pub role_permissions: HashMap<String, Vec<Permission>>,
}

impl PermissionScheme {
    /// Create a new permission scheme
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
                "Permission {}.{} already exists for role {}",
                permission.resource_type, permission.permission_type, role
            )));
        }

        // Add the permission
        permissions.push(permission);
        Ok(())
    }

    /// Remove a permission from a role
    pub fn remove_permission(
        &mut self,
        role: &str,
        resource_type: &str,
        permission_type: &PermissionType,
    ) -> Result<()> {
        if self.is_system {
            return Err(Error::Entity(
                "Cannot modify a system permission scheme".to_string(),
            ));
        }

        let role_idx = self.role_permissions.iter().position(|(r, _)| r == role);

        if role_idx.is_some() {
            let perm_idx = self.role_permissions[role].iter().position(|p| {
                p.resource_type == resource_type && &p.permission_type == permission_type
            });

            if let Some(perm_idx) = perm_idx {
                self.role_permissions
                    .get_mut(role)
                    .unwrap()
                    .remove(perm_idx);
                Ok(())
            } else {
                Err(Error::Entity(format!(
                    "Permission {}.{} not found for role {}",
                    resource_type, permission_type, role
                )))
            }
        } else {
            Err(Error::Entity(format!("Role {} not found", role)))
        }
    }

    /// Check if a role has a specific permission
    pub fn has_permission(
        &self,
        role: &str,
        resource_type: &str,
        permission_type: &PermissionType,
    ) -> bool {
        if let Some(permissions) = self.role_permissions.get(role) {
            permissions
                .iter()
                .any(|p| p.resource_type == resource_type && p.permission_type == *permission_type)
        } else {
            false
        }
    }

    /// Get all permissions for a role
    pub fn get_role_permissions(&self, role: &str) -> Option<&Vec<Permission>> {
        self.role_permissions.get(role)
    }

    /// Create a default admin permission scheme
    pub fn create_admin_scheme() -> Self {
        let mut scheme = Self::new("Admin Scheme".to_string());
        scheme.description = Some("Default permission scheme for administrators".to_string());

        // Add permissions for super admin
        let admin_permissions = vec![Permission {
            resource_type: "*".to_string(),
            permission_type: PermissionType::Admin,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        }];

        scheme
            .role_permissions
            .insert("SuperAdmin".to_string(), admin_permissions);

        // Add permissions for regular admin
        let editor_permissions = vec![
            Permission {
                resource_type: "*".to_string(),
                permission_type: PermissionType::Read,
                access_level: AccessLevel::All,
                resource_uuids: vec![],
                constraints: None,
            },
            Permission {
                resource_type: "*".to_string(),
                permission_type: PermissionType::Create,
                access_level: AccessLevel::All,
                resource_uuids: vec![],
                constraints: None,
            },
            Permission {
                resource_type: "*".to_string(),
                permission_type: PermissionType::Update,
                access_level: AccessLevel::All,
                resource_uuids: vec![],
                constraints: None,
            },
            Permission {
                resource_type: "*".to_string(),
                permission_type: PermissionType::Delete,
                access_level: AccessLevel::All,
                resource_uuids: vec![],
                constraints: None,
            },
        ];

        scheme
            .role_permissions
            .insert("Admin".to_string(), editor_permissions);

        // Add permissions for viewer
        let viewer_permissions = vec![Permission {
            resource_type: "*".to_string(),
            permission_type: PermissionType::Read,
            access_level: AccessLevel::All,
            resource_uuids: vec![],
            constraints: None,
        }];

        scheme
            .role_permissions
            .insert("Viewer".to_string(), viewer_permissions);

        scheme.is_system = true;
        scheme
    }
}

impl Display for PermissionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionType::Read => write!(f, "Read"),
            PermissionType::Create => write!(f, "Create"),
            PermissionType::Update => write!(f, "Update"),
            PermissionType::Delete => write!(f, "Delete"),
            PermissionType::Publish => write!(f, "Publish"),
            PermissionType::Admin => write!(f, "Admin"),
            PermissionType::Execute => write!(f, "Execute"),
            PermissionType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

impl FromRow<'_, PgRow> for PermissionScheme {
    fn from_row(row: &PgRow) -> std::result::Result<Self, sqlx::Error> {
        // Extract base entity fields
        let uuid_str = row.try_get::<String, _>("uuid")?;
        let uuid = uuid::Uuid::parse_str(&uuid_str).map_err(|e| sqlx::Error::ColumnDecode {
            index: "uuid".to_string(),
            source: Box::new(e),
        })?;

        let base = AbstractRDataEntity {
            uuid,
            path: row.try_get("path")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            created_by: row.try_get("created_by").unwrap_or_else(|_| Uuid::nil()),
            updated_by: row.try_get("updated_by").ok(),
            published: row.try_get("published").unwrap_or(false),
            version: row.try_get("version").unwrap_or(1),
            custom_fields: HashMap::new(), // We can populate this if needed
        };

        // Extract main fields
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description").ok();
        let is_system: bool = row.try_get("is_system").unwrap_or(false);

        // Extract JSON data for role_permissions
        let role_permissions: HashMap<String, Vec<Permission>> =
            match row.try_get::<serde_json::Value, _>("role_permissions") {
                Ok(json) => serde_json::from_value(json).unwrap_or_default(),
                Err(_) => HashMap::new(),
            };

        Ok(PermissionScheme {
            base,
            name,
            description,
            is_system,
            role_permissions,
        })
    }
}
