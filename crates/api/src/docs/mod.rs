#![allow(clippy::needless_for_each)] // utoipa derive macros generate for_each calls

use actix_web::web;
use actix_web::HttpResponse;
use serde_json;
use utoipa::openapi::schema::Type;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{ObjectBuilder, OpenApi as UtoipaOpenApi, SchemaFormat};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::{Config, SwaggerUi};

/// Admin API Documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::admin::auth::routes::admin_login,
        crate::admin::auth::routes::admin_register,
        crate::admin::auth::routes::admin_logout,
        crate::admin::auth::routes::admin_refresh_token,
        crate::admin::auth::routes::admin_revoke_all_tokens,
        crate::health::admin_health_check,
        crate::admin::entity_definitions::routes::list_entity_definitions,
        crate::admin::entity_definitions::routes::get_entity_definition,
        crate::admin::entity_definitions::routes::create_entity_definition,
        crate::admin::entity_definitions::routes::update_entity_definition,
        crate::admin::entity_definitions::routes::delete_entity_definition,
        crate::admin::entity_definitions::routes::apply_entity_definition_schema,
        crate::admin::api_keys::routes::create_api_key,
        crate::admin::api_keys::routes::list_api_keys,
        crate::admin::api_keys::routes::revoke_api_key,
        crate::admin::workflows::routes::list::list_workflows,
        crate::admin::workflows::routes::crud::get_workflow_details,
        crate::admin::workflows::routes::crud::create_workflow,
        crate::admin::workflows::routes::crud::update_workflow,
        crate::admin::workflows::routes::crud::delete_workflow,
        crate::admin::workflows::routes::runs::run_workflow_now,
        crate::admin::workflows::routes::runs::run_workflow_now_upload,
        crate::admin::workflows::routes::list::list_workflow_runs,
        crate::admin::workflows::routes::runs::list_workflow_run_logs,
        crate::admin::workflows::routes::list::list_all_workflow_runs,
        crate::admin::workflows::routes::cron::cron_preview,
        crate::admin::workflows::routes::versions::list_workflow_versions,
        crate::admin::workflows::routes::versions::get_workflow_version,
        crate::admin::entity_definitions::routes::list_entity_definition_versions,
        crate::admin::entity_definitions::routes::get_entity_definition_version,
        crate::admin::dsl::routes::validate_dsl,
        crate::admin::dsl::routes::list_from_options,
        crate::admin::dsl::routes::list_to_options,
        crate::admin::dsl::routes::list_transform_options,
        crate::admin::system::routes::get_entity_versioning_settings,
        crate::admin::system::routes::update_entity_versioning_settings,
        crate::admin::permissions::routes::list_roles,
        crate::admin::permissions::routes::get_role,
        crate::admin::permissions::routes::create_role,
        crate::admin::permissions::routes::update_role,
        crate::admin::permissions::routes::delete_role,
        crate::admin::permissions::routes::assign_roles_to_user,
        crate::admin::permissions::routes::assign_roles_to_api_key,
        crate::admin::users::routes::list_users,
        crate::admin::users::routes::get_user,
        crate::admin::users::routes::create_user,
        crate::admin::users::routes::update_user,
        crate::admin::users::routes::delete_user,
        crate::admin::users::routes::get_user_roles,
        crate::admin::users::routes::assign_roles_to_user,
        crate::admin::auth::routes::get_user_permissions,
        crate::admin::meta::routes::get_dashboard_stats,
    ),
    components(
        schemas(
            crate::models::HealthData,
            crate::admin::entity_definitions::models::EntityDefinitionSchema,
            crate::admin::entity_definitions::models::PathUuid,
            crate::admin::entity_definitions::models::ApplySchemaRequest,
            crate::admin::api_keys::models::CreateApiKeyRequest,
            crate::admin::api_keys::models::ApiKeyResponse,
            crate::admin::api_keys::models::ApiKeyCreatedResponse,
            crate::admin::api_keys::models::ReassignApiKeyRequest,
            crate::query::PaginationQuery,
            crate::admin::auth::models::AdminLoginRequest,
            crate::admin::auth::models::AdminLoginResponse,
            crate::admin::auth::models::AdminRegisterRequest,
            crate::admin::auth::models::AdminRegisterResponse,
            crate::admin::auth::models::EmptyRequest,
            crate::admin::auth::models::RefreshTokenRequest,
            crate::admin::auth::models::RefreshTokenResponse,
            crate::admin::auth::models::LogoutRequest,
            crate::admin::entity_definitions::models::PaginationQuery,
            crate::admin::entity_definitions::models::PathUuid,
            crate::admin::entity_definitions::models::EntityDefinitionSchema,
            crate::admin::entity_definitions::models::FieldDefinitionSchema,
            crate::admin::entity_definitions::models::FieldTypeSchema,
            crate::admin::entity_definitions::models::UiSettingsSchema,
            crate::admin::entity_definitions::models::OptionsSourceSchema,
            crate::admin::entity_definitions::models::SelectOptionSchema,
            crate::admin::entity_definitions::models::EntityDefinitionListResponse,
            crate::admin::entity_definitions::models::ApplySchemaRequest,
            crate::admin::entity_definitions::models::FieldConstraints,
            crate::admin::entity_definitions::models::StringConstraints,
            crate::admin::entity_definitions::models::NumericConstraints,
            crate::admin::entity_definitions::models::DateTimeConstraints,
            crate::admin::entity_definitions::models::SelectConstraints,
            crate::admin::entity_definitions::models::RelationConstraints,
            crate::admin::entity_definitions::models::SchemaConstraints,
            crate::admin::api_keys::models::CreateApiKeyRequest,
            crate::admin::api_keys::models::ApiKeyResponse,
            crate::admin::api_keys::models::ApiKeyCreatedResponse,
            crate::admin::auth::models::EmptyRequest,
            crate::admin::workflows::models::WorkflowSummary,
            crate::admin::workflows::models::CreateWorkflowRequest,
            crate::admin::workflows::models::UpdateWorkflowRequest,
            crate::admin::workflows::models::CreateWorkflowResponse,
            crate::admin::workflows::models::WorkflowDetail,
            crate::admin::workflows::models::WorkflowRunSummary,
            crate::admin::workflows::models::WorkflowRunLogDto,
            crate::admin::workflows::models::WorkflowRunUpload,
            crate::admin::workflows::models::WorkflowVersionMeta,
            crate::admin::workflows::models::WorkflowVersionPayload,
            crate::admin::entity_definitions::models::EntityDefinitionVersionMeta,
            crate::admin::entity_definitions::models::EntityDefinitionVersionPayload,
            crate::admin::dsl::models::DslValidateRequest,
            crate::admin::dsl::models::DslValidateResponse,
            crate::admin::dsl::models::DslFieldSpec,
            crate::admin::dsl::models::DslTypeSpec,
            crate::admin::dsl::models::DslOptionsResponse,
            crate::admin::system::models::EntityVersioningSettingsDto,
            crate::admin::system::models::UpdateSettingsBody,
            crate::admin::permissions::models::RoleResponse,
            crate::admin::permissions::models::CreateRoleRequest,
            crate::admin::permissions::models::UpdateRoleRequest,
            crate::admin::permissions::models::AssignRolesRequest,
            crate::admin::permissions::models::PermissionResponse,
            crate::admin::meta::models::DashboardStats,
            crate::admin::meta::models::EntityStats,
            crate::admin::meta::models::EntityTypeCount,
            crate::admin::meta::models::WorkflowStats,
            crate::admin::meta::models::WorkflowWithLatestStatus,
            crate::admin::users::models::UserResponse,
            crate::admin::users::models::CreateUserRequest,
            crate::admin::users::models::UpdateUserRequest
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
        (name = "permissions", description = "Role management"),
        (name = "users", description = "User management"),
        (name = "meta", description = "Dashboard metadata and statistics"),
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
                    .schema_type(Type::String)
                    .format(Some(SchemaFormat::Custom("uuid".to_owned())))
                    .description(Some("UUID string"))
                    .build()
                    .into(),
            );
        }
    }
}

/// Custom schema for `time::OffsetDateTime`
pub struct DateTimeSchemaAddon;

impl Modify for DateTimeSchemaAddon {
    fn modify(&self, openapi: &mut UtoipaOpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // Add the OffsetDateTime schema
            components.schemas.insert(
                "OffsetDateTime".to_owned(),
                ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(SchemaFormat::Custom("date-time".to_owned())))
                    .description(Some("ISO 8601 date and time with offset"))
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
                    let new_key = key.split('.').next_back().unwrap_or(key).to_string();
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

/// Custom schema for `serde_json::Value`
pub struct JsonValueSchemaAddon;

impl Modify for JsonValueSchemaAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // Create a simpler schema for JSON Value
            let schema = utoipa::openapi::Schema::Object(
                ObjectBuilder::new().description(Some("JSON value")).build(),
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
        crate::health::public_health_check,
        crate::public::entities::routes::list_available_entities,
        crate::public::entities::routes::list_by_path,
        crate::public::queries::routes::query_entities,
        crate::public::dynamic_entities::routes::list_entities,
        crate::public::dynamic_entities::routes::create_entity,
        crate::public::dynamic_entities::routes::get_entity,
        crate::public::dynamic_entities::routes::update_entity,
        crate::public::dynamic_entities::routes::delete_entity,
        crate::public::workflows::routes::get_workflow_data,
        crate::public::workflows::routes::trigger_workflow,
        crate::public::workflows::routes::get_workflow_stats,
        crate::public::workflows::routes::post_workflow_ingest,
        crate::public::entities::routes::list_entity_versions,
        crate::public::entities::routes::get_entity_version
    ),
    components(
        schemas(
            crate::models::HealthData,
            crate::public::entities::models::EntityTypeInfo,
            crate::public::entities::models::EntityQuery,
            crate::public::entities::models::BrowseKind,
            crate::public::entities::models::BrowseNode,
            crate::public::queries::models::AdvancedEntityQuery,
            crate::query::PaginationQuery,
            crate::query::StandardQuery,
            crate::public::dynamic_entities::models::DynamicEntityResponse,
            crate::public::dynamic_entities::models::EntityResponse,
            crate::public::entities::models::VersionMeta,
            crate::public::entities::models::VersionPayload
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

/// Generate the `OpenAPI` specification for admin endpoints
fn generate_admin_openapi_spec() -> utoipa::openapi::OpenApi {
    AdminApiDoc::openapi()
}

/// Generate the `OpenAPI` specification for public endpoints
fn generate_public_openapi_spec() -> utoipa::openapi::OpenApi {
    PublicApiDoc::openapi()
}

/// Admin `OpenAPI` documentation endpoint
pub async fn admin_openapi_json() -> HttpResponse {
    HttpResponse::Ok().json(generate_admin_openapi_spec())
}

/// Public `OpenAPI` documentation endpoint
pub async fn public_openapi_json() -> HttpResponse {
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
