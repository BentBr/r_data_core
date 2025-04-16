use actix_web::web;
use actix_web::{get, HttpRequest, HttpResponse};
use serde_json;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi};

/// Generate API documentation for admin endpoints
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::admin::auth::admin_login,
        crate::api::admin::auth::admin_register,
        crate::api::admin::auth::admin_logout,
        crate::api::admin::class_definitions::routes::list_class_definitions,
        crate::api::admin::class_definitions::routes::get_class_definition,
        crate::api::admin::class_definitions::routes::create_class_definition,
        crate::api::admin::class_definitions::routes::update_class_definition,
        crate::api::admin::class_definitions::routes::delete_class_definition
    ),
    components(
        schemas(
            crate::api::admin::auth::AdminLoginRequest,
            crate::api::admin::auth::AdminLoginResponse,
            crate::api::admin::auth::AdminRegisterRequest,
            crate::api::admin::auth::AdminRegisterResponse,
            crate::api::admin::class_definitions::models::PaginationQuery,
            crate::api::admin::class_definitions::models::PathUuid,
            // crate::entity::ClassDefinition
        )
    ),
    tags(
        (name = "admin-auth", description = "Admin authentication endpoints"),
        (name = "admin", description = "Administrative endpoints")
    ),
    info(
        title = "R Data Core Admin API",
        version = "0.1.0",
        description = "Administrative interface for the Digital Asset Management backend",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8888", description = "Local development server")
    )
)]
struct AdminApiDoc;

/// Generate API documentation for public endpoints
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::docs::test_endpoint,
        crate::api::public::entities::routes::list_available_entities,
        crate::api::public::entities::routes::get_entity,
        crate::api::public::queries::routes::query_entities
    ),
    components(
        schemas(
            crate::api::public::entities::models::EntityTypeInfo,
            crate::api::public::entities::models::EntityQuery,
            crate::api::public::queries::models::AdvancedEntityQuery,
            // crate::entity::DynamicEntity
        )
    ),
    tags(
        (name = "public", description = "Public API endpoints")
    ),
    info(
        title = "R Data Core Public API",
        version = "0.1.0",
        description = "Public API for the Digital Asset Management backend",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8888", description = "Local development server")
    )
)]
struct PublicApiDoc;

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

/// Generate the OpenAPI specification for admin endpoints
fn generate_admin_openapi_spec() -> utoipa::openapi::OpenApi {
    AdminApiDoc::openapi()
}

/// Generate the OpenAPI specification for public endpoints
fn generate_public_openapi_spec() -> utoipa::openapi::OpenApi {
    PublicApiDoc::openapi()
}

/// Admin OpenAPI documentation endpoint
pub async fn admin_openapi_json(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().json(generate_admin_openapi_spec())
}

/// Public OpenAPI documentation endpoint
pub async fn public_openapi_json(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().json(generate_public_openapi_spec())
}

/// Register documentation routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    log::debug!("Registering documentation routes");
    cfg.service(test_endpoint);

    // Configure SwaggerUI for admin API
    cfg.service(
        SwaggerUi::new("/admin/api/docs/{_:.*}")
            .url(
                "/admin/api/docs/openapi.json",
                generate_admin_openapi_spec(),
            )
            .config(
                Config::default()
                    .try_it_out_enabled(true)
                    .display_request_duration(true)
                    .deep_linking(true)
                    .filter(true),
            ),
    );
    log::debug!("Registered Admin Swagger UI at /admin/api/docs/");

    // Configure SwaggerUI for public API
    cfg.service(
        SwaggerUi::new("/api/docs/{_:.*}")
            .url("/api/docs/openapi.json", generate_public_openapi_spec())
            .config(
                Config::default()
                    .try_it_out_enabled(true)
                    .display_request_duration(true)
                    .deep_linking(true)
                    .filter(true),
            ),
    );
    log::debug!("Registered Public Swagger UI at /api/docs/");

    // Register OpenAPI JSON endpoints
    cfg.route(
        "/admin/api/docs/openapi.json",
        web::get().to(admin_openapi_json),
    );
    cfg.route("/api/docs/openapi.json", web::get().to(public_openapi_json));
}
