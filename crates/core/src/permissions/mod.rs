#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod permission_scheme;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;
use permission_scheme::{AccessLevel, Permission, PermissionType, ResourceNamespace};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionContext {
    pub user_uuid: Uuid,
    pub organization_uuid: Option<Uuid>,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionScope {
    Entity {
        entity_type: String,
        entity_uuid: Option<Uuid>,
    },
    Workflow {
        workflow_uuid: Option<Uuid>,
    },
    System,
}

#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn get_permissions_for_roles(&self, roles: &[String]) -> Result<Vec<Permission>>;
}

#[async_trait]
pub trait PermissionChecker: Send + Sync {
    async fn is_allowed(
        &self,
        ctx: &PermissionContext,
        scope: &PermissionScope,
        action: &PermissionType,
    ) -> Result<bool>;
}

pub struct DefaultPermissionService<R: PermissionRepository> {
    repository: R,
}

impl<R: PermissionRepository> DefaultPermissionService<R> {
    /// Create a new default permission service
    ///
    /// # Arguments
    /// * `repository` - Permission repository implementation
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<R: PermissionRepository> PermissionChecker for DefaultPermissionService<R> {
    async fn is_allowed(
        &self,
        ctx: &PermissionContext,
        scope: &PermissionScope,
        action: &PermissionType,
    ) -> Result<bool> {
        let perms = self
            .repository
            .get_permissions_for_roles(&ctx.roles)
            .await?;
        let matches_action = |p: &Permission| &p.permission_type == action;
        let within_scope = |p: &Permission| {
            matches!(
                scope,
                PermissionScope::System | PermissionScope::Workflow { .. }
            ) || {
                if let PermissionScope::Entity { entity_uuid, .. } = scope {
                    // For entity scope, permission must be for Entities namespace
                    // Note: The old code checked resource_type != "*" && resource_type != entity_type
                    // With enum, we check if it's Entities namespace
                    // Entity type matching would need to be done via constraints or resource_uuids
                    if !matches!(p.resource_type, ResourceNamespace::Entities) {
                        return false;
                    }
                    matches!(
                        &p.access_level,
                        AccessLevel::All | AccessLevel::Own | AccessLevel::Group
                    ) || entity_uuid.is_none()
                } else {
                    false
                }
            }
        };
        Ok(perms.iter().any(|p| matches_action(p) && within_scope(p)))
    }
}
