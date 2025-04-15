use actix_web::web;
use actix_web::{HttpRequest, HttpResponse, get};
use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Config};
use serde_json;

/// Generate API documentation using OpenAPI
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::admin::list_class_definitions,
        crate::api::docs::test_endpoint
    ),
    components(
        schemas(crate::api::admin::PaginationQuery, crate::api::admin::PathUuid)
    ),
    tags(
        (name = "admin", description = "Administrative endpoints"),
        (name = "public", description = "Public API endpoints"),
        (name = "auth", description = "Authentication endpoints")
    ),
    info(
        title = "R Data Core API",
        version = "0.1.0",
        description = "A Digital Asset Management backend providing efficient data handling and distribution",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8888", description = "Local development server")
    )
)]
struct ApiDoc;

/// Test endpoint to verify the server is working
#[utoipa::path(
    get,
    path = "/test",
    responses(
        (status = 200, description = "Test endpoint working", body = String)
    )
)]
#[get("/test")]
pub async fn test_endpoint() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "message": "API is working!"
    }))
}

/// Generate the OpenAPI specification
fn generate_openapi_spec() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

/// OpenAPI documentation endpoint
pub async fn openapi_json(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().json(generate_openapi_spec())
}

/// Register documentation routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    log::debug!("Registering documentation routes");
    cfg.service(test_endpoint);
    
    // Configure SwaggerUI with enhanced options
    cfg.service(
        SwaggerUi::new("/swagger-ui/{_:.*}")
            .url("/api-docs/openapi.json", generate_openapi_spec())
            .config(
                Config::default()
                    .try_it_out_enabled(true)
                    .display_request_duration(true)
                    .deep_linking(true)
                    .filter(true)
            )
    );
    log::debug!("Registered Swagger UI at /swagger-ui/");
    
    cfg.route("/api-docs/openapi.json", web::get().to(openapi_json));
    log::debug!("Registered OpenAPI JSON at /api-docs/openapi.json");
} 