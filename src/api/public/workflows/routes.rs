use actix_web::{get, web, HttpResponse, Responder};
use uuid::Uuid;

use crate::api::public::workflows::repository;
use crate::api::ApiState;

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_workflow_data).service(get_workflow_schema);
}

#[get("/workflows/{uuid}")]
pub async fn get_workflow_data(
    path: web::Path<Uuid>,
    state: web::Data<ApiState>,
) -> impl Responder {
    // Stub: Return 501 for now; implementation will use adapters and query mapping
    let uuid = path.into_inner();
    let _cfg = match repository::get_provider_config(&state.db_pool, uuid).await {
        Ok(Some(cfg)) => cfg,
        Ok(None) => return HttpResponse::NotFound().finish(),
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    HttpResponse::NotImplemented().finish()
}

#[get("/workflows/{uuid}/schema")]
pub async fn get_workflow_schema(_path: web::Path<Uuid>) -> impl Responder {
    // Stub schema endpoint
    HttpResponse::NotImplemented().finish()
}
