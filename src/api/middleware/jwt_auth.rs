use actix_web::{
    web,
    dev::{forward_ready, Service, Transform, ServiceRequest, ServiceResponse},
    Error,
    error::{ErrorUnauthorized},
    HttpMessage,
    http::header,
};
use std::future::{ready, Ready};
use std::rc::Rc;
use futures::future::LocalBoxFuture;

use crate::api::auth::{verify_jwt, AuthUserClaims};
use crate::api::ApiState;

/// JWT authentication middleware
pub struct JwtAuth;

impl JwtAuth {
    /// Create a new instance of JwtAuth middleware
    pub fn new() -> Self {
        Self {}
    }
}

/// Middleware service for JWT auth
pub struct JwtAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Transform<S, ServiceRequest> for JwtAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = JwtAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware { 
            service: Rc::new(service)
        }))
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
        let service = self.service.clone();
        
        // Clone what we need from the request
        let state_opt = req.app_data::<web::Data<ApiState>>().cloned();
        let auth_header_str = req.headers()
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        Box::pin(async move {
            // Verify state is available
            let state = match state_opt {
                Some(state) => state,
                None => return Err(ErrorUnauthorized("Missing application state")),
            };
            
            // Get JWT secret
            let jwt_secret = &state.jwt_secret;
            
            // Verify auth header is present and parse it
            let auth_str = match auth_header_str {
                Some(s) => s,
                None => return Err(ErrorUnauthorized("Authorization header missing")),
            };
            
            // Check for Bearer prefix
            if !auth_str.starts_with("Bearer ") {
                return Err(ErrorUnauthorized("Invalid authorization header format"));
            }
            
            let token = &auth_str[7..]; // Remove "Bearer " prefix
            
            // Verify JWT token
            match verify_jwt(token, jwt_secret) {
                Ok(claims) => {
                    // Add user claims to request extensions
                    req.extensions_mut().insert(AuthUserClaims {
                        sub: claims.sub,
                        name: claims.name,
                        email: claims.email,
                        is_admin: claims.is_admin,
                        exp: claims.exp,
                        iat: claims.iat
                    });
                    
                    // Continue with the request
                    service.call(req).await
                },
                Err(_) => Err(ErrorUnauthorized("Invalid or expired token")),
            }
        })
    }
} 