#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{
    dev::Payload, error::ErrorForbidden, Error, FromRequest, HttpRequest, HttpResponse,
};
use futures::future::{ready, Ready};
use log::debug;
use r_data_core_core::permissions::permission_scheme::{PermissionType, ResourceNamespace};

use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::ApiResponse;

/// Extractor that requires a specific permission
///
/// This extractor combines authentication (`RequiredAuth`) with permission checking.
/// If the user doesn't have the required permission, it returns a 403 Forbidden response.
///
/// # Example
/// ```rust,ignore
/// async fn my_route(
///     auth: PermissionRequired<{ ResourceNamespace::Workflows }, { PermissionType::Read }>,
/// ) -> impl Responder {
///     // User is authenticated and has permission
/// }
/// ```
pub struct PermissionRequired {
    pub auth: RequiredAuth,
    pub namespace: ResourceNamespace,
    pub permission_type: PermissionType,
    pub path: Option<String>,
}

impl PermissionRequired {
    /// Create a new `PermissionRequired` extractor
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // RequiredAuth is not const-constructible
    pub fn new(
        auth: RequiredAuth,
        namespace: ResourceNamespace,
        permission_type: PermissionType,
        path: Option<String>,
    ) -> Self {
        Self {
            auth,
            namespace,
            permission_type,
            path,
        }
    }
}

impl FromRequest for PermissionRequired {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // This should never be called directly - use the macro instead
        ready(Err(ErrorForbidden(
            "PermissionRequired must be used with the permission_required! macro",
        )))
    }
}

/// Macro to create a `PermissionRequired` extractor
///
/// # Usage
/// ```rust,ignore
/// #[get("/workflows")]
/// async fn list_workflows(
///     permission: PermissionRequired<Workflows, Read>,
/// ) -> impl Responder {
///     // Handler code
/// }
/// ```
#[macro_export]
macro_rules! permission_required {
    ($namespace:ident, $permission_type:ident) => {
        $crate::auth::permission_required::PermissionRequired
    };
}

/// Helper function to check permission and return appropriate response
///
/// # Errors
/// Returns an error response if the user doesn't have the required permission
pub fn check_permission_and_respond(
    auth: &RequiredAuth,
    namespace: &ResourceNamespace,
    permission_type: &PermissionType,
    path: Option<&str>,
) -> Result<(), HttpResponse> {
    if !permission_check::has_permission(&auth.0, namespace, permission_type, path) {
        debug!(
            "Permission denied: user '{}' does not have {}:{} permission",
            auth.0.name,
            namespace.as_str(),
            permission_type
        );
        return Err(ApiResponse::<()>::forbidden(&format!(
            "Insufficient permissions to perform {} on {}",
            permission_type,
            namespace.as_str()
        )));
    }
    Ok(())
}

/// Extension trait for `RequiredAuth` to add permission checking
pub trait RequiredAuthExt {
    /// Check if the authenticated user has a specific `permission`
    fn has_permission(
        &self,
        namespace: &ResourceNamespace,
        permission_type: &PermissionType,
        path: Option<&str>,
    ) -> bool;

    /// Check permission and return an error response if denied
    /// Returns an `ApiResponse` that can be used as a `Responder`
    ///
    /// # Errors
    /// Returns an error response if the user doesn't have the required permission
    fn require_permission(
        &self,
        namespace: &ResourceNamespace,
        permission_type: &PermissionType,
        path: Option<&str>,
    ) -> Result<(), actix_web::HttpResponse>;
}

impl RequiredAuthExt for RequiredAuth {
    fn has_permission(
        &self,
        namespace: &ResourceNamespace,
        permission_type: &PermissionType,
        path: Option<&str>,
    ) -> bool {
        permission_check::has_permission(&self.0, namespace, permission_type, path)
    }

    fn require_permission(
        &self,
        namespace: &ResourceNamespace,
        permission_type: &PermissionType,
        path: Option<&str>,
    ) -> Result<(), actix_web::HttpResponse> {
        if !permission_check::has_permission(&self.0, namespace, permission_type, path) {
            return Err(ApiResponse::<()>::forbidden(&format!(
                "Insufficient permissions to perform {} on {}",
                permission_type,
                namespace.as_str()
            )));
        }
        Ok(())
    }
}
