use actix_web::{HttpResponse, Responder, get};
use utoipa_swagger_ui::SwaggerUi;
use serde::Serialize;

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
    info: String
}

/// Swagger UI for API documentation
#[get("/api/docs/admin/openapi.json")]
pub async fn admin_api_docs() -> impl Responder {
    HttpResponse::Ok().json(ApiDoc { 
        info: "API documentation coming soon".to_string() 
    })
}

/// Swagger UI for Entities API
#[get("/api/docs/entities/openapi.json")]
pub async fn entities_api_docs() -> impl Responder {
    HttpResponse::Ok().json(ApiDoc { 
        info: "API documentation coming soon".to_string() 
    })
}

/// Register Swagger UI routes
pub fn register_routes() -> SwaggerUi {
    // FIXME: Once OpenAPI is fully implemented, replace this with proper configuration
    let ui = SwaggerUi::new("/api/docs/{_:.*}");
    // We're not using the .url() method since it requires OpenApi
    ui
} 