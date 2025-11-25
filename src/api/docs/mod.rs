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
        r_data_core_api::admin::auth::routes::admin_login,
        r_data_core_api::admin::auth::routes::admin_register,
        r_data_core_api::admin::auth::routes::admin_logout,
        r_data_core_api::admin::auth::routes::admin_refresh_token,
        r_data_core_api::admin::auth::routes::admin_revoke_all_tokens,
        crate::api::health::admin_health_check,
        r_data_core_api::admin::entity_definitions::routes::list_entity_definitions,
        r_data_core_api::admin::entity_definitions::routes::get_entity_definition,
        r_data_core_api::admin::entity_definitions::routes::create_entity_definition,
        r_data_core_api::admin::entity_definitions::routes::update_entity_definition,
        r_data_core_api::admin::entity_definitions::routes::delete_entity_definition,
        r_data_core_api::admin::entity_definitions::routes::apply_entity_definition_schema,
        r_data_core_api::admin::api_keys::routes::create_api_key,
        r_data_core_api::admin::api_keys::routes::list_api_keys,
        r_data_core_api::admin::api_keys::routes::revoke_api_key,
        r_data_core_api::admin::workflows::routes::list::list_workflows,
        r_data_core_api::admin::workflows::routes::crud::get_workflow_details,
        r_data_core_api::admin::workflows::routes::crud::create_workflow,
        r_data_core_api::admin::workflows::routes::crud::update_workflow,
        r_data_core_api::admin::workflows::routes::crud::delete_workflow,
        r_data_core_api::admin::workflows::routes::runs::run_workflow_now,
        r_data_core_api::admin::workflows::routes::runs::run_workflow_now_upload,
        r_data_core_api::admin::workflows::routes::list::list_workflow_runs,
        r_data_core_api::admin::workflows::routes::runs::list_workflow_run_logs,
        r_data_core_api::admin::workflows::routes::list::list_all_workflow_runs,
        r_data_core_api::admin::workflows::routes::cron::cron_preview,
        r_data_core_api::admin::workflows::routes::versions::list_workflow_versions,
        r_data_core_api::admin::workflows::routes::versions::get_workflow_version,
        r_data_core_api::admin::entity_definitions::routes::list_entity_definition_versions,
        r_data_core_api::admin::entity_definitions::routes::get_entity_definition_version,
        r_data_core_api::admin::dsl::routes::validate_dsl,
        r_data_core_api::admin::dsl::routes::list_from_options,
        r_data_core_api::admin::dsl::routes::list_to_options,
        r_data_core_api::admin::dsl::routes::list_transform_options,
        r_data_core_api::admin::system::routes::get_entity_versioning_settings,
        r_data_core_api::admin::system::routes::update_entity_versioning_settings,
    ),
    components(
        schemas(
            r_data_core_api::models::HealthData,
            r_data_core_api::admin::entity_definitions::models::EntityDefinitionSchema,
            r_data_core_api::admin::entity_definitions::models::PathUuid,
            r_data_core_api::admin::entity_definitions::models::ApplySchemaRequest,
            r_data_core_api::admin::api_keys::models::CreateApiKeyRequest,
            r_data_core_api::admin::api_keys::models::ApiKeyResponse,
            r_data_core_api::admin::api_keys::models::ApiKeyCreatedResponse,
            r_data_core_api::admin::api_keys::models::ReassignApiKeyRequest,
            r_data_core_api::query::PaginationQuery,
            r_data_core_api::admin::auth::models::AdminLoginRequest,
            r_data_core_api::admin::auth::models::AdminLoginResponse,
            r_data_core_api::admin::auth::models::AdminRegisterRequest,
            r_data_core_api::admin::auth::models::AdminRegisterResponse,
            r_data_core_api::admin::auth::models::EmptyRequest,
            r_data_core_api::admin::auth::models::RefreshTokenRequest,
            r_data_core_api::admin::auth::models::RefreshTokenResponse,
            r_data_core_api::admin::auth::models::LogoutRequest,
            r_data_core_api::admin::entity_definitions::models::PaginationQuery,
            r_data_core_api::admin::entity_definitions::models::PathUuid,
            r_data_core_api::admin::entity_definitions::models::EntityDefinitionSchema,
            r_data_core_api::admin::entity_definitions::models::FieldDefinitionSchema,
            r_data_core_api::admin::entity_definitions::models::FieldTypeSchema,
            r_data_core_api::admin::entity_definitions::models::UiSettingsSchema,
            r_data_core_api::admin::entity_definitions::models::OptionsSourceSchema,
            r_data_core_api::admin::entity_definitions::models::SelectOptionSchema,
            r_data_core_api::admin::entity_definitions::models::EntityDefinitionListResponse,
            r_data_core_api::admin::entity_definitions::models::ApplySchemaRequest,
            r_data_core_api::admin::entity_definitions::models::FieldConstraints,
            r_data_core_api::admin::entity_definitions::models::StringConstraints,
            r_data_core_api::admin::entity_definitions::models::NumericConstraints,
            r_data_core_api::admin::entity_definitions::models::DateTimeConstraints,
            r_data_core_api::admin::entity_definitions::models::SelectConstraints,
            r_data_core_api::admin::entity_definitions::models::RelationConstraints,
            r_data_core_api::admin::entity_definitions::models::SchemaConstraints,
            r_data_core_api::admin::api_keys::models::CreateApiKeyRequest,
            r_data_core_api::admin::api_keys::models::ApiKeyResponse,
            r_data_core_api::admin::api_keys::models::ApiKeyCreatedResponse,
            r_data_core_api::admin::auth::models::EmptyRequest,
            r_data_core_api::admin::workflows::models::WorkflowSummary,
            r_data_core_api::admin::workflows::models::CreateWorkflowRequest,
            r_data_core_api::admin::workflows::models::UpdateWorkflowRequest,
            r_data_core_api::admin::workflows::models::CreateWorkflowResponse,
            r_data_core_api::admin::workflows::models::WorkflowDetail,
            r_data_core_api::admin::workflows::models::WorkflowRunSummary,
            r_data_core_api::admin::workflows::models::WorkflowRunLogDto,
            r_data_core_api::admin::workflows::models::WorkflowRunUpload,
            r_data_core_api::admin::workflows::models::WorkflowVersionMeta,
            r_data_core_api::admin::workflows::models::WorkflowVersionPayload,
            r_data_core_api::admin::entity_definitions::models::EntityDefinitionVersionMeta,
            r_data_core_api::admin::entity_definitions::models::EntityDefinitionVersionPayload,
            r_data_core_api::admin::dsl::models::DslValidateRequest,
            r_data_core_api::admin::dsl::models::DslValidateResponse,
            r_data_core_api::admin::dsl::models::DslFieldSpec,
            r_data_core_api::admin::dsl::models::DslTypeSpec,
            r_data_core_api::admin::dsl::models::DslOptionsResponse,
            r_data_core_api::admin::system::models::EntityVersioningSettingsDto,
            r_data_core_api::admin::system::models::UpdateSettingsBody
        )
    ),
    modifiers(&SecurityAddon, &UuidSchemaAddon, &DateTimeSchemaAddon, &ModelSchemaAddon, &JsonValueSchemaAddon),
    tags(
        (name = "admin-health", description = "Admin health check endpoints"),
        (name = "admin-auth", description = "Admin authentication endpoints"),
        (name = "entity-definitions", description = "Entity definition management"),
        (name = "api-keys", description = "API key management"),
        (name = "workflows", description = "Workflow management"),
        (name = "DSL", description = "Workflow DSL validation and options"),
        (name = "system", description = "System settings management"),
    ),
    info(
        title = "R Data Core Admin API",
        version = "0.1.0",
        description = "Admin API for the Master Data Management with flexible pagination support",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://rdatacore.docker", description = "Local development server from image"),
        (url = "http://localhost:8888", description = "Local development server from local build")
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
                if key.contains("crate.api.admin.entity_definitions.models.") {
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
        crate::api::public::entities::routes::list_by_path,
        crate::api::public::queries::routes::query_entities,
        crate::api::public::dynamic_entities::routes::list_entities,
        crate::api::public::dynamic_entities::routes::create_entity,
        crate::api::public::dynamic_entities::routes::get_entity,
        crate::api::public::dynamic_entities::routes::update_entity,
        crate::api::public::dynamic_entities::routes::delete_entity,
        crate::api::public::workflows::routes::get_workflow_data,
        crate::api::public::workflows::routes::get_workflow_stats,
        crate::api::public::workflows::routes::post_workflow_ingest,
        crate::api::public::entities::routes::list_entity_versions,
        crate::api::public::entities::routes::get_entity_version
    ),
    components(
        schemas(
            r_data_core_api::models::HealthData,
            r_data_core_api::public::entities::models::EntityTypeInfo,
            r_data_core_api::public::entities::models::EntityQuery,
            r_data_core_api::public::entities::models::BrowseKind,
            r_data_core_api::public::entities::models::BrowseNode,
            r_data_core_api::public::queries::models::AdvancedEntityQuery,
            r_data_core_api::query::PaginationQuery,
            r_data_core_api::query::StandardQuery,
            r_data_core_api::public::dynamic_entities::models::DynamicEntityResponse,
            r_data_core_api::public::dynamic_entities::models::EntityResponse,
            r_data_core_api::public::entities::models::VersionMeta,
            r_data_core_api::public::entities::models::VersionPayload
        )
    ),
    modifiers(&SecurityAddon, &UuidSchemaAddon, &DateTimeSchemaAddon, &ModelSchemaAddon, &JsonValueSchemaAddon),
    tags(
        (name = "public-health", description = "Public health check endpoints"),
        (name = "public", description = "Public API endpoints"),
        (name = "dynamic-entities", description = "Dynamic entity CRUD operations"),
        (name = "workflows", description = "Workflow provider and consumer endpoints")
    ),
    info(
        title = "R Data Core Public API",
        version = "0.1.0",
        description = "Public API for the Master Data Management with flexible pagination support",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://rdatacore.docker", description = "Local development server from image"),
        (url = "http://localhost:8888", description = "Local development server from local build")
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
