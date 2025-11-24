use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    Error,
};
use futures::future::LocalBoxFuture;
use std::rc::Rc;

pub trait AuthMiddlewareService<S, B>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    fn process_auth(
        &self,
        req: ServiceRequest,
        service: Rc<S>,
    ) -> LocalBoxFuture<'static, Result<ServiceResponse<B>, Error>>;
}
