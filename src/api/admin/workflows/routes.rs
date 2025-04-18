use actix_web::{web, HttpResponse, Responder};
use serde_json::json;

/// Register workflow routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("").route(web::get().to(list_workflows)));
}

/// List available workflows
pub async fn list_workflows() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": Vec::<String>::new(),
        "message": "No workflows available"
    }))
}
