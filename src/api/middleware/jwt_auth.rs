use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse},
    error::ErrorUnauthorized,
    web, Error, HttpMessage,
};
use futures::future::LocalBoxFuture;
use std::rc::Rc;

use crate::api::auth::extract_and_validate_jwt;
use crate::api::middleware::AuthMiddlewareService;
use crate::api::ApiState;

/// Middleware service for JWT auth
pub struct JwtAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> AuthMiddlewareService<S, B> for JwtAuthMiddleware<S>
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
        let request = req.request().clone();
        let state_result = req.app_data::<web::Data<ApiState>>().cloned();
        let service_clone = service.clone();

        Box::pin(async move {
            let state = match state_result {
                Some(state) => state,
                None => {
                    log::error!("Failed to extract API state from request");
                    return Err(ErrorUnauthorized("Missing application state"));
                }
            };

            let jwt_secret = &state.jwt_secret;

            match extract_and_validate_jwt(&request, jwt_secret).await {
                Ok(Some(claims)) => {
                    // Add claims to request extensions
                    req.extensions_mut().insert(claims);

                    // Always proceed to the handler - auth enforcement happens at handler level now
                    service_clone.call(req).await
                }
                Ok(None) => {
                    // No JWT token found or invalid token
                    // Let the handler decide whether this is acceptable
                    service_clone.call(req).await
                }
                Err(e) => {
                    log::error!("JWT validation error: {:?}", e);
                    service_clone.call(req).await
                }
            }
        })
    }
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddleware<S>
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
        log::debug!("JwtAuthMiddleware processing path: {}", req.path());

        // For all other paths, use process_auth to handle authentication
        self.process_auth(req, self.service.clone())
    }
}
