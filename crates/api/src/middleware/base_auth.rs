use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    web, Error,
};
use futures::future::LocalBoxFuture;
use std::rc::Rc;

use crate::api_state::ApiStateWrapper;

/// Base Authentication Middleware Service
/// Provides common functionality for authentication middleware
pub trait AuthMiddlewareService<S, B>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    /// Get API state from the request
    fn get_state(&self, req: &ServiceRequest) -> Result<web::Data<ApiStateWrapper>, Error> {
        log::debug!(
            "AuthMiddlewareService::get_state called for path: {}",
            req.path()
        );
        let state = req.app_data::<web::Data<ApiStateWrapper>>().cloned();

        if let Some(ref s) = state {
            log::debug!("API state found successfully");
            Ok(s.clone())
        } else {
            log::error!("Failed to find API state in request");
            Err(actix_web::error::ErrorUnauthorized(
                "Missing application state",
            ))
        }
    }

    /// Process the authentication and call the inner service
    fn process_auth(
        &self,
        req: ServiceRequest,
        service: Rc<S>,
    ) -> LocalBoxFuture<'static, Result<ServiceResponse<B>, Error>>;
}
