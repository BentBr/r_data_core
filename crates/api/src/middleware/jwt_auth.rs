#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse},
    error::ErrorUnauthorized,
    web, Error, HttpMessage,
};
use futures::future::LocalBoxFuture;
use std::rc::Rc;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::extract_and_validate_jwt;
use crate::middleware::base_auth::AuthMiddlewareService;

/// Middleware service for JWT auth
#[allow(dead_code)]
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
        let state_result = req.app_data::<web::Data<ApiStateWrapper>>().cloned();
        let service_clone = Rc::clone(&service);

        Box::pin(async move {
            let Some(state) = state_result else {
                log::error!("Failed to extract API state from request");
                return Err(ErrorUnauthorized("Missing application state"));
            };

            let jwt_secret = state.jwt_secret();

            // Only the side effect differs per outcome; the request always goes
            // through, because auth enforcement happens at the handler level.
            match extract_and_validate_jwt(&request, jwt_secret).await {
                Ok(Some(claims)) => {
                    req.extensions_mut().insert(claims);
                }
                // No JWT token found, or one we could not validate. The handler
                // decides whether that is acceptable.
                Ok(None) => {}
                Err(e) => log::error!("JWT validation error: {e:?}"),
            }

            service_clone.call(req).await
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
        let path = req.path();
        log::debug!("JwtAuthMiddleware processing path: {path}");

        // For all other paths, use process_auth to handle authentication
        self.process_auth(req, self.service.clone())
    }
}
