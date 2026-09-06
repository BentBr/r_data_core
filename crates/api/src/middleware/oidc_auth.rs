#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Accepting bearer tokens from an external identity provider.
//!
//! This is not a parallel authorization path. It populates `AuthUserClaims`
//! in request extensions and stops — exactly what the local JWT arm does —
//! so every route already guarded by `RequiredAuth` or a permission check
//! keeps its existing behaviour with no SSO-specific branches. The whole
//! feature is a third way to answer "who is this", never a second answer to
//! "what may they do".
//!
//! **Ordering matters.** The middleware decides whether a token is its
//! business by peeking at the unverified `iss` claim. A token that names
//! another issuer — a local `RDataCore` JWT, say — passes through untouched
//! for the other arms to handle. A token that names *our* issuer and then
//! fails is refused outright rather than passed on: falling through would
//! turn a rejected identity into an unauthenticated request, and some routes
//! answer those.

use std::rc::Rc;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::{ErrorForbidden, ErrorServiceUnavailable, ErrorUnauthorized},
    web, Error, HttpMessage,
};
use futures::future::{ready, LocalBoxFuture, Ready};

use r_data_core_core::admin_jwt::ACCESS_TOKEN_EXPIRY_SECONDS;
use r_data_core_services::OidcAuthError;

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use crate::auth::extract_jwt_token_string;

/// Middleware accepting identity-provider bearer tokens.
///
/// Inert unless `RDC_OIDC_ISSUER` is configured, so registering it
/// unconditionally costs a pointer check per request.
#[derive(Default)]
pub struct OidcAuth;

impl OidcAuth {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

pub struct OidcAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Transform<S, ServiceRequest> for OidcAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = OidcAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(OidcAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

impl<S, B> Service<ServiceRequest> for OidcAuthMiddleware<S>
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
        let service = Rc::clone(&self.service);

        // Decided before the future so the borrow of `req` ends here.
        let runtime = req
            .app_data::<web::Data<ApiStateWrapper>>()
            .and_then(|state| state.oidc_runtime().cloned());
        let token = extract_jwt_token_string(req.request()).map(str::to_string);

        Box::pin(async move {
            // Not configured, or nothing that looks like a bearer token.
            let (Some(runtime), Some(token)) = (runtime, token) else {
                return service.call(req).await;
            };

            // Someone else's token. Passing it through is the point: the local
            // JWT arm still has to see it.
            if !runtime.token_targets_this_issuer(&token) {
                return service.call(req).await;
            }

            let now = time::OffsetDateTime::now_utc().unix_timestamp();
            match runtime
                .authenticate(&token, now, ACCESS_TOKEN_EXPIRY_SECONDS)
                .await
            {
                Ok(claims) => {
                    req.extensions_mut().insert(claims);
                    service.call(req).await
                }
                Err(error) => Err(refusal(&error)),
            }
        })
    }
}

/// Turn an authentication failure into the right kind of refusal.
///
/// The distinction that matters is 403 versus 503. A locked account and an
/// unreachable identity provider are entirely different operational problems,
/// and collapsing them sends whoever is on call to the wrong place.
fn refusal(error: &OidcAuthError) -> Error {
    // Logged in full; the client is told far less. Which part of a forgery
    // was wrong is not something an attacker should learn from the response.
    log::warn!("OIDC authentication failed: {error}");

    match error {
        OidcAuthError::Denied(_) => ErrorForbidden("Not permitted to sign in"),
        OidcAuthError::Rejected(_) => ErrorUnauthorized("Invalid token"),
        OidcAuthError::Keys(_) | OidcAuthError::Unavailable(_) => {
            ErrorServiceUnavailable("Authentication is temporarily unavailable")
        }
    }
}
