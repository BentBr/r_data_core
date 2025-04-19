use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    web, Error, HttpMessage,
};
use futures::future::{ready, LocalBoxFuture, Ready};
use std::rc::Rc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::api::auth::{extract_and_validate_api_key, extract_and_validate_jwt};
use crate::api::middleware::AuthMiddlewareService;
use crate::api::ApiState;

/// Combined Authentication middleware for JWT and API Keys
pub struct CombinedAuth;

impl Default for CombinedAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl CombinedAuth {
    /// Create a new instance of the combined authentication middleware
    pub fn new() -> Self {
        Self {}
    }
}

/// Middleware service for combined auth
pub struct CombinedAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Transform<S, ServiceRequest> for CombinedAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = CombinedAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CombinedAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

impl<S, B> AuthMiddlewareService<S, B> for CombinedAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    fn process_auth(
        &self,
        req: ServiceRequest,
        service: Rc<S>,
    ) -> LocalBoxFuture<'static, Result<ServiceResponse<B>, Error>> {
        // Extract what we need from the request before creating the future
        let request = req.request().clone();
        let path = request.path().to_string();

        // Get the API state
        let state_result = req.app_data::<web::Data<ApiState>>().cloned();
        let service_clone = service.clone();

        Box::pin(async move {
            // Handle state extraction
            let state = match state_result {
                Some(state) => state,
                None => {
                    log::error!(
                        "Failed to extract API state from request for path: {}",
                        path
                    );
                    return Err(ErrorUnauthorized("Missing application state"));
                }
            };

            // Try JWT authentication first
            let jwt_result = extract_and_validate_jwt(&request, &state.jwt_secret).await;
            match jwt_result {
                Ok(Some(claims)) => {
                    // Add user claims to request extensions
                    req.extensions_mut().insert(claims);

                    // Set auth method for context
                    req.extensions_mut().insert(AuthMethod::Jwt);

                    // Continue with the request
                    return service_clone.call(req).await;
                }
                Ok(None) => {
                    // JWT authentication failed, continue to API key
                    log::debug!(
                        "JWT authentication failed, trying API key for path: {}",
                        path
                    );
                }
                Err(e) => {
                    log::debug!("JWT authentication error: {:?}", e);
                    // Continue to API key auth even on JWT error
                }
            }

            // Try API key authentication if JWT failed
            let api_key_result = extract_and_validate_api_key(&request, &state.db_pool).await;
            match api_key_result {
                Ok(Some((key, user_uuid))) => {
                    let key_uuid = key.uuid;

                    req.extensions_mut().insert(ApiKeyInfo {
                        uuid: key_uuid,
                        user_uuid,
                        name: key.name,
                        created_at: key.created_at,
                        expires_at: key.expires_at,
                    });

                    // Set auth method for context
                    req.extensions_mut().insert(AuthMethod::ApiKey);

                    // Continue with the request
                    return service_clone.call(req).await;
                }
                Ok(None) => {
                    // API key authentication failed
                    log::debug!(
                        "Authentication failed for path {}: both JWT and API key auth failed",
                        path
                    );
                }
                Err(e) => {
                    log::debug!("API key authentication error: {:?}", e);
                }
            }

            // Authentication failed, return an error that will be handled by the global error handler
            // This ensures proper handling by the standard Actix error handlers
            let error_message =
                "Authentication required. Please provide a valid JWT token or API key.";
            log::debug!(
                "Authentication failed for path {}: {}",
                request.path(),
                error_message
            );

            Err(ErrorUnauthorized(error_message))
        })
    }
}

impl<S, B> Service<ServiceRequest> for CombinedAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path();
        log::debug!("CombinedAuthMiddleware processing path: {}", path);

        // Process authentication for all paths
        // The decision about which paths need auth is made at the route registration level
        self.process_auth(req, self.service.clone())
    }
}

/// Authentication method used for this request
#[derive(Debug, Clone, Copy)]
pub enum AuthMethod {
    Jwt,
    ApiKey,
}

/// API key info attached to the request
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub uuid: Uuid,
    pub user_uuid: Uuid,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
}
