use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::{dev::{Transform, Service, ServiceRequest, ServiceResponse, forward_ready}, Error, FromRequest, HttpMessage, web};
use futures::future::LocalBoxFuture;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Deserialize;
use log::debug;
use chrono::Utc;

use crate::api::ApiState;
use crate::api::auth::AuthUserClaims;

#[derive(Debug, Deserialize)]
pub struct ApiKeyClaims {
    pub user_id: i64,
    pub api_key_id: i64,
}

pub struct ApiAuth;

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
    type Transform = ApiAuthMiddleware<S>;
    type InitError = ();
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

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        
        Box::pin(async move {
            // Get JWT secret from app state
            let state = req.app_data::<web::Data<ApiState>>().unwrap();
            let jwt_secret = &state.jwt_secret;
            let pool = &state.db_pool;
            
            // Try JWT authentication first
            if let Some(auth_header) = req.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = auth_str.trim_start_matches("Bearer ").trim();
                        
                        // Validate JWT token
                        let validation = Validation::default();
                        match decode::<Claims>(
                            token,
                            &DecodingKey::from_secret(jwt_secret.as_bytes()),
                            &validation,
                        ) {
                            Ok(token_data) => {
                                // Add user claims to request extensions
                                req.extensions_mut().insert(Claims {
                                    sub: token_data.claims.sub,
                                    exp: token_data.claims.exp,
                                    iat: token_data.claims.iat,
                                    role: token_data.claims.role,
                                    permissions: token_data.claims.permissions,
                                });
                                
                                return service.call(req).await;
                            }
                            Err(e) => {
                                debug!("JWT validation failed: {}", e);
                                // Continue to try API key authentication
                            }
                        }
                    }
                }
            }
            
            // Try API key authentication
            if let Some(api_key_header) = req.headers().get("X-API-Key") {
                if let Ok(api_key) = api_key_header.to_str() {
                    // Check if API key is valid
                    let api_key_result = sqlx::query!(
                        r#"
                        SELECT a.id, a.user_id, a.is_active, a.expires_at, u.is_active as user_is_active
                        FROM api_keys a
                        JOIN admin_users u ON a.user_id = u.id
                        WHERE a.api_key = $1
                        "#,
                        api_key
                    )
                    .fetch_optional(pool)
                    .await;
                    
                    match api_key_result {
                        Ok(Some(row)) => {
                            // Check if API key and user are active
                            if !row.is_active || !row.user_is_active {
                                return Err(ErrorUnauthorized("Invalid API key"));
                            }
                            
                            // Check if API key is expired
                            if let Some(expires_at) = row.expires_at {
                                if expires_at < chrono::Utc::now() {
                                    return Err(ErrorUnauthorized("API key expired"));
                                }
                            }
                            
                            // Update last_used_at (don't block the request for this)
                            let api_key_id = row.id;
                            let now = chrono::Utc::now();
                            tokio::spawn(async move {
                                let _ = sqlx::query!(
                                    "UPDATE api_keys SET last_used_at = $1 WHERE id = $2",
                                    now,
                                    api_key_id
                                )
                                .execute(pool)
                                .await;
                            });
                            
                            // Add API key claims to request extensions
                            req.extensions_mut().insert(ApiKeyClaims {
                                user_id: row.user_id,
                                api_key_id: row.id,
                            });
                            
                            return service.call(req).await;
                        }
                        Ok(None) => {
                            return Err(ErrorUnauthorized("Invalid API key"));
                        }
                        Err(_) => {
                            return Err(ErrorUnauthorized("Error validating API key"));
                        }
                    }
                }
            }
            
            // If neither JWT nor API key authentication succeeded
            Err(ErrorUnauthorized("Authentication required"))
        })
    }
} 