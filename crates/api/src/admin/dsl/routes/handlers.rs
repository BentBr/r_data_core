#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::api_state::{ApiStateTrait, ApiStateWrapper};
use actix_web::{get, post, web, Responder};

use crate::admin::dsl::models::{
    DslOptionsAndExamplesResponse, DslOptionsResponse, DslValidateRequest, DslValidateResponse,
};
use crate::admin::dsl::routes::diagnostics;
use crate::admin::dsl::routes::options_builders::{
    build_from_type_specs, build_to_type_specs, build_transform_type_specs,
};
use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::ApiResponse;
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_workflow::dsl::{
    ArithmeticOp, ArithmeticTransform, AuthenticateTransform, ConcatTransform, DslProgram,
    EntityFilter, EntityWriteMode, FormatConfig, FromDef, Operand, OutputMode, SourceConfig,
    StringOperand, ToDef, Transform,
};

#[utoipa::path(
    post,
    path = "/admin/api/v1/dsl/validate",
    tag = "DSL",
    request_body = DslValidateRequest,
    responses(
        (status = 200, description = "DSL is valid", body = DslValidateResponse),
        (status = 422, description = "Invalid DSL", body = Value),
        (status = 500, description = "Internal server error")
    ),
    security(("jwt" = []))
)]
#[post("/validate")]
pub async fn validate_dsl(
    payload: web::Json<DslValidateRequest>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission (DSL validation is part of workflow management)
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to validate DSL");
    }

    // Parse failures and rule failures both come back located: see
    // `diagnostics` for why the path matters more than the message.
    let steps = match diagnostics::parse_steps(&payload.steps) {
        Ok(steps) => steps,
        Err(violations) => {
            return ApiResponse::<()>::unprocessable_entity_with_violations(
                "Invalid DSL",
                violations,
            )
        }
    };

    let program = DslProgram {
        steps,
        on_complete: None,
    };
    match program.validate() {
        Ok(()) => ApiResponse::ok(DslValidateResponse { valid: true }),
        Err(e) => ApiResponse::<()>::unprocessable_entity_with_violations(
            "Invalid DSL",
            vec![diagnostics::semantic_violation(e.to_string())],
        ),
    }
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/dsl/from/options",
    tag = "DSL",
    responses(
        (status = 200, description = "Available FROM types and field specs", body = DslOptionsAndExamplesResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("jwt" = []))
)]
#[allow(clippy::unwrap_used)] // serializing hardcoded literals — infallible
#[get("/from/options")]
pub async fn list_from_options(auth: RequiredAuth) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view DSL options");
    }

    let types = DslOptionsResponse {
        types: build_from_type_specs(),
    };
    let ex_csv = serde_json::to_value(FromDef::Format {
        source: SourceConfig {
            source_type: "uri".to_string(),
            config: serde_json::json!({
                "uri": "http://example.com/data.csv"
            }),
            auth: None,
        },
        format: FormatConfig {
            format_type: "csv".to_string(),
            options: serde_json::json!({
                "has_header": true,
                "delimiter": ","
            }),
        },
        mapping: std::iter::once(("price".to_string(), "price".to_string())).collect(),
    })
    .unwrap();
    let ex_json = serde_json::to_value(FromDef::Format {
        source: SourceConfig {
            source_type: "uri".to_string(),
            config: serde_json::json!({
                "uri": "http://example.com/data.json"
            }),
            auth: None,
        },
        format: FormatConfig {
            format_type: "json".to_string(),
            options: serde_json::json!({}),
        },
        mapping: std::iter::once(("amount".to_string(), "amount".to_string())).collect(),
    })
    .unwrap();
    let ex_entity = serde_json::to_value(FromDef::Entity {
        entity_definition: "product".to_string(),
        filter: Some(EntityFilter {
            field: "sku".to_string(),
            operator: "=".to_string(),
            value: "ABC-001".to_string(),
        }),
        mapping: std::iter::once(("price".to_string(), "price".to_string())).collect(),
    })
    .unwrap();
    let resp = DslOptionsAndExamplesResponse {
        types: types.types,
        examples: vec![ex_csv, ex_json, ex_entity],
    };
    ApiResponse::ok(resp)
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/dsl/to/options",
    tag = "DSL",
    responses(
        (status = 200, description = "Available TO types and field specs", body = DslOptionsAndExamplesResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("jwt" = []))
)]
#[allow(clippy::unwrap_used)] // serializing hardcoded literals — infallible
#[get("/to/options")]
pub async fn list_to_options(
    data: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view DSL options");
    }

    let workflow_mail_configured = data.workflow_service().mail_service.is_some();
    let types = DslOptionsResponse {
        types: build_to_type_specs(workflow_mail_configured),
    };
    let ex_csv = serde_json::to_value(ToDef::Format {
        output: OutputMode::Api,
        format: FormatConfig {
            format_type: "csv".to_string(),
            options: serde_json::json!({
                "has_header": true,
                "delimiter": ","
            }),
        },
        mapping: std::iter::once(("price".to_string(), "entity.total".to_string())).collect(),
    })
    .unwrap();
    let ex_json = serde_json::to_value(ToDef::Format {
        output: OutputMode::Api,
        format: FormatConfig {
            format_type: "json".to_string(),
            options: serde_json::json!({}),
        },
        mapping: std::iter::once(("price".to_string(), "entity.total".to_string())).collect(),
    })
    .unwrap();
    let ex_entity = serde_json::to_value(ToDef::Entity {
        entity_definition: "product".to_string(),
        path: Some("/".to_string()),
        mode: EntityWriteMode::Create,
        identify: None,
        update_key: None,
        mapping: std::iter::once(("price".to_string(), "price".to_string())).collect(),
    })
    .unwrap();
    let resp = DslOptionsAndExamplesResponse {
        types: types.types,
        examples: vec![ex_csv, ex_json, ex_entity],
    };
    ApiResponse::ok(resp)
}

#[utoipa::path(
    get,
    path = "/admin/api/v1/dsl/transform/options",
    tag = "DSL",
    responses(
        (status = 200, description = "Available TRANSFORM types and field specs", body = DslOptionsAndExamplesResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("jwt" = []))
)]
#[allow(clippy::unwrap_used)] // serializing hardcoded literals — infallible
#[get("/transform/options")]
pub async fn list_transform_options(
    data: web::Data<ApiStateWrapper>,
    auth: RequiredAuth,
) -> impl Responder {
    // Check permission
    if !permission_check::has_permission(
        &auth.0,
        &ResourceNamespace::Workflows,
        &PermissionType::Read,
        None,
    ) {
        return ApiResponse::<()>::forbidden("Insufficient permissions to view DSL options");
    }

    let workflow_mail_configured = data.workflow_service().mail_service.is_some();
    let types = DslOptionsResponse {
        types: build_transform_type_specs(workflow_mail_configured),
    };
    let ex_arith = serde_json::to_value(Transform::Arithmetic(ArithmeticTransform {
        target: "price".to_string(),
        left: Operand::Field {
            field: "price".to_string(),
        },
        op: ArithmeticOp::Add,
        right: Operand::Const { value: 5.0 },
    }))
    .unwrap();
    let ex_concat = serde_json::to_value(Transform::Concat(ConcatTransform {
        target: "full_name".to_string(),
        left: StringOperand::Field {
            field: "first_name".to_string(),
        },
        separator: Some(" ".to_string()),
        right: StringOperand::Field {
            field: "last_name".to_string(),
        },
    }))
    .unwrap();
    let mut extra_claims = std::collections::HashMap::new();
    extra_claims.insert("role".to_string(), "role".to_string());
    let ex_auth = serde_json::to_value(Transform::Authenticate(AuthenticateTransform {
        entity_type: "user".to_string(),
        identifier_field: "email".to_string(),
        password_field: "password".to_string(),
        input_identifier: "identifier".to_string(),
        input_password: "password".to_string(),
        target_token: "token".to_string(),
        extra_claims,
        token_expiry_seconds: None,
    }))
    .unwrap();
    let resp = DslOptionsAndExamplesResponse {
        types: types.types,
        examples: vec![ex_arith, ex_concat, ex_auth],
    };
    ApiResponse::ok(resp)
}
