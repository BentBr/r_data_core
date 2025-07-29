use actix_web::web;
use actix_web::{HttpRequest, HttpResponse};
use serde_json;
use serde_json::json;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{ObjectBuilder, OpenApi as UtoipaOpenApi, SchemaFormat, SchemaType};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::{Config, SwaggerUi};

/// Admin API Documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::admin::auth::admin_login,
        crate::api::admin::auth::admin_register,
        crate::api::admin::auth::admin_logout,
        crate::api::health::admin_health_check,
        crate::api::admin::class_definitions::routes::list_class_definitions,
        crate::api::admin::class_definitions::routes::get_class_definition,
        crate::api::admin::class_definitions::routes::create_class_definition,
        crate::api::admin::class_definitions::routes::update_class_definition,
        crate::api::admin::class_definitions::routes::delete_class_definition,
        crate::api::admin::class_definitions::routes::apply_class_definition_schema,
        crate::api::admin::api_keys::routes::create_api_key,
        crate::api::admin::api_keys::routes::list_api_keys,
        crate::api::admin::api_keys::routes::revoke_api_key
    ),
    components(
        schemas(
            crate::api::models::HealthData,
            crate::api::admin::auth::AdminLoginRequest,
            crate::api::admin::auth::AdminLoginResponse,
            crate::api::admin::auth::AdminRegisterRequest,
            crate::api::admin::auth::AdminRegisterResponse,
            crate::api::admin::auth::EmptyRequest,
            crate::api::admin::class_definitions::models::PaginationQuery,
            crate::api::admin::class_definitions::models::PathUuid,
            crate::api::admin::class_definitions::models::ClassDefinitionSchema,
            crate::api::admin::class_definitions::models::FieldDefinitionSchema,
            crate::api::admin::class_definitions::models::FieldTypeSchema,
            crate::api::admin::class_definitions::models::UiSettingsSchema,
            crate::api::admin::class_definitions::models::OptionsSourceSchema,
            crate::api::admin::class_definitions::models::SelectOptionSchema,
            crate::api::admin::class_definitions::models::ClassDefinitionListResponse,
            crate::api::admin::class_definitions::models::ApplySchemaRequest,
            crate::api::admin::class_definitions::models::FieldConstraints,
            crate::api::admin::class_definitions::models::StringConstraints,
            crate::api::admin::class_definitions::models::NumericConstraints,
            crate::api::admin::class_definitions::models::DateTimeConstraints,
            crate::api::admin::class_definitions::models::SelectConstraints,
            crate::api::admin::class_definitions::models::RelationConstraints,
            crate::api::admin::class_definitions::models::SchemaConstraints,
            crate::api::admin::api_keys::routes::CreateApiKeyRequest,
            crate::api::admin::api_keys::routes::ApiKeyResponse,
            crate::api::admin::api_keys::routes::ApiKeyCreatedResponse
        )
    ),
    modifiers(&SecurityAddon, &UuidSchemaAddon, &DateTimeSchemaAddon, &ModelSchemaAddon),
    tags(
        (name = "admin-auth", description = "Admin authentication endpoints"),
        (name = "admin-health", description = "Admin health check endpoints"),
        (name = "admin", description = "Administrative endpoints for managing system resources"),
        (name = "class-definitions", description = "Endpoints for managing entity type definitions and structure"),
        (name = "api-keys", description = "Endpoints for managing API keys")
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

/// Add a modifier for security scheme
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut UtoipaOpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // Add JWT Bearer authentication
            components.add_security_scheme(
                "jwt",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );

            // Add API Key authentication
            // The simplified way - using serde_json to build the schema
            let api_key_scheme = serde_json::json!({
                "type": "apiKey",
                "name": "X-API-Key",
                "in": "header",
                "description": "API Key for accessing the API"
            });

            components.security_schemes.insert(
                "apiKey".to_string(),
                serde_json::from_value(api_key_scheme).unwrap(),
            );
        }
    }
}

/// Custom schema for UUID
pub struct UuidSchemaAddon;

impl Modify for UuidSchemaAddon {
    fn modify(&self, openapi: &mut UtoipaOpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.schemas.insert(
                "Uuid".to_owned(),
                ObjectBuilder::new()
                    .schema_type(SchemaType::String)
                    .format(Some(SchemaFormat::Custom("uuid".to_owned())))
                    .description(Some("UUID string"))
                    .example(Some(json!("550e8400-e29b-41d4-a716-446655440000")))
                    .build()
                    .into(),
            );
        }
    }
}

/// Custom schema for time::OffsetDateTime
pub struct DateTimeSchemaAddon;

impl Modify for DateTimeSchemaAddon {
    fn modify(&self, openapi: &mut UtoipaOpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // Add the OffsetDateTime schema
            components.schemas.insert(
                "OffsetDateTime".to_owned(),
                ObjectBuilder::new()
                    .schema_type(SchemaType::String)
                    .format(Some(SchemaFormat::Custom("date-time".to_owned())))
                    .description(Some("ISO 8601 date and time with offset"))
                    .example(Some(json!("2023-01-01T12:00:00Z")))
                    .build()
                    .into(),
            );
        }
    }
}

/// Custom schema for Models
pub struct ModelSchemaAddon;

impl Modify for ModelSchemaAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // Find schema references with fully qualified paths and rename them
            let mut schemas_to_rename = Vec::new();

            // First collect the keys that need to be renamed
            for key in components.schemas.keys() {
                if key.contains("crate.api.admin.class_definitions.models.") {
                    let new_key = key.split('.').last().unwrap_or(key).to_string();
                    schemas_to_rename.push((key.clone(), new_key));
                }
            }

            // Then perform the renames
            for (old_key, new_key) in schemas_to_rename {
                if let Some(schema) = components.schemas.remove(&old_key) {
                    components.schemas.insert(new_key, schema);
                }
            }
        }
    }
}

/// Custom schema for serde_json::Value
pub struct JsonValueSchemaAddon;

impl Modify for JsonValueSchemaAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // Create a simpler schema for JSON Value
            let schema = utoipa::openapi::Schema::Object(
                ObjectBuilder::new()
                    .schema_type(SchemaType::String)
                    .format(Some(SchemaFormat::Custom("json".to_owned())))
                    .description(Some("JSON value"))
                    .example(Some(serde_json::json!({"example": "value"})))
                    .build(),
            );

            components
                .schemas
                .insert("Value".to_owned(), utoipa::openapi::RefOr::T(schema));
        }
    }
}

/// Public API Documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::health::public_health_check,
        crate::api::public::entities::routes::list_available_entities,
        crate::api::public::queries::routes::query_entities,
        crate::api::public::dynamic_entities::routes::list_entities,
        crate::api::public::dynamic_entities::routes::create_entity,
        crate::api::public::dynamic_entities::routes::get_entity,
        crate::api::public::dynamic_entities::routes::update_entity,
        crate::api::public::dynamic_entities::routes::delete_entity
    ),
    components(
        schemas(
            crate::api::models::HealthData,
            crate::api::public::entities::models::EntityTypeInfo,
            crate::api::public::entities::models::EntityQuery,
            crate::api::public::queries::models::AdvancedEntityQuery,
            crate::api::query::PaginationQuery,
            crate::api::public::dynamic_entities::routes::DynamicEntityResponse,
            crate::api::public::dynamic_entities::routes::EntityResponse
        )
    ),
    modifiers(&SecurityAddon, &UuidSchemaAddon, &DateTimeSchemaAddon, &ModelSchemaAddon, &JsonValueSchemaAddon),
    tags(
        (name = "public-health", description = "Public health check endpoints"),
        (name = "public", description = "Public API endpoints"),
        (name = "dynamic-entities", description = "Dynamic entity CRUD operations")
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
                    .filter(true)
                    .persist_authorization(true), // Remember auth between page refreshes
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
                    .filter(true)
                    .persist_authorization(true), // Remember auth between page refreshes
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
