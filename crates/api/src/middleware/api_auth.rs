#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    web, Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use serde::Deserialize;

use crate::api_state::ApiStateWrapper;
use crate::auth::extract_and_validate_api_key;
use crate::middleware::base_auth::AuthMiddlewareService;
use crate::middleware::combined_auth::ApiKeyInfo;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ApiKeyClaims {
    pub user_uuid: i64,
    pub api_key_uuid: i64,
}

#[allow(dead_code)] // Middleware type for future use
pub struct ApiAuth;

impl Default for ApiAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiAuth {
    #[allow(dead_code)] // Middleware method for future use
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

#[allow(dead_code)] // Middleware type for future use
pub struct ApiAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> AuthMiddlewareService<S, B> for ApiAuthMiddleware<S>
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
        // Extract what we need before moving into the future
        let request = req.request().clone();

        // Get the API state before moving into the future
        let state_result = req.app_data::<web::Data<ApiStateWrapper>>().cloned();
        let service_clone = service.clone();

        Box::pin(async move {
            // Handle state extraction
            let Some(_state) = state_result else {
                return Err(ErrorUnauthorized("Missing application state"));
            };

            // Try API key authentication
            if let Ok(Some((key, user_uuid))) = extract_and_validate_api_key(&request).await {
                log::debug!(
                    "API key authentication successful, user_uuid: {user_uuid}"
                );
                log::debug!("API key UUID: {:?}", key.uuid);

                // Add API key info to request extensions
                let key_uuid = key.uuid;
                log::debug!("Using API key UUID: {key_uuid}");

                req.extensions_mut().insert(ApiKeyInfo {
                    uuid: key_uuid,
                    user_uuid,
                    name: key.name,
                    created_at: key.created_at,
                    expires_at: key.expires_at,
                });
                log::debug!("API key info inserted into request extensions");

                // Continue with the request
                log::debug!("Continuing with request after API key auth");
                return service_clone.call(req).await;
            }

            // API key authentication failed
            Err(ErrorUnauthorized("Valid API key required"))
        })
    }
}

impl<S, B> Service<ServiceRequest> for ApiAuthMiddleware<S>
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
        self.process_auth(req, self.service.clone())
    }
}
