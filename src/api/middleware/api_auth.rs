use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use serde::Deserialize;

use crate::api::ApiState;

#[derive(Debug, Deserialize)]
pub struct ApiKeyClaims {
    pub user_uuid: i64,
    pub api_key_uuid: i64,
}

pub struct ApiAuth;

impl Default for ApiAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiAuth {
    pub fn new() -> Self {
        ApiAuth
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

pub struct ApiAuthMiddleware<S> {
    service: Rc<S>,
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
        let service = self.service.clone();

        Box::pin(async move {
            let state = req.app_data::<actix_web::web::Data<ApiState>>().unwrap();
            let auth_header = req.headers().get("Authorization");

            if let Some(auth_header) = auth_header {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = auth_str[7..].to_string();
                        let api_key: Option<ApiKeyClaims> = state
                            .cache_manager
                            .get(&format!("api_key:{}", token))
                            .await
                            .map_err(actix_web::Error::from)?;

                        if let Some(api_key) = api_key {
                            req.extensions_mut().insert(api_key);
                            return service.call(req).await;
                        }
                    }
                }
            }

            service.call(req).await
        })
    }
}
