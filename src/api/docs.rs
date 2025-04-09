use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;
use utoipa_swagger_ui::SwaggerUi;

/// API docs for admin API routes
// FIXME: Uncomment when all path handlers are properly annotated
// #[derive(OpenApi)]
#[derive(Serialize)]
pub struct AdminApiDoc {
    /// API info
    pub info: String,
}

// Entities API Documentation
// FIXME: Uncomment when all path handlers are properly annotated
// #[derive(OpenApi)]
#[derive(Serialize)]
pub struct EntitiesApiDoc {
    /// API info
    pub info: String,
}

/// API docs for public API routes
// FIXME: Uncomment when all path handlers are properly annotated
// #[derive(OpenApi)]
#[derive(Serialize)]
pub struct PublicApiDoc {
    /// API info
    pub info: String,
}

#[derive(Serialize)]
pub struct ApiDoc {
    info: String,
}

/// Swagger UI for API documentation
#[get("/api/docs/admin/openapi.json")]
pub async fn admin_api_docs() -> impl Responder {
    HttpResponse::Ok().json(ApiDoc {
        info: "API documentation coming soon".to_string(),
    })
}

/// Swagger UI for Entities API
#[get("/api/docs/entities/openapi.json")]
pub async fn entities_api_docs() -> impl Responder {
    HttpResponse::Ok().json(ApiDoc {
        info: "API documentation coming soon".to_string(),
    })
}

/// Register Swagger UI routes
/// This function can be conditionally called from main.rs based on configuration
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    // Register API documentation endpoints
    cfg.service(admin_api_docs);
    cfg.service(entities_api_docs);
    
    // Register Swagger UI
    let ui = SwaggerUi::new("/api/docs/{_:.*}");
    cfg.service(ui);
}
