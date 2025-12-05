#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use actix_web::{get, post, web, Responder};

use crate::auth::auth_enum::RequiredAuth;
use crate::auth::permission_check;
use crate::response::{ApiResponse, ValidationViolation};
use r_data_core_core::permissions::role::{PermissionType, ResourceNamespace};
use r_data_core_workflow::dsl::{
    ArithmeticOp, ArithmeticTransform, ConcatTransform, DslProgram, DslStep, EntityFilter,
    EntityWriteMode, FormatConfig, FromDef, Operand, OutputMode, SourceConfig, StringOperand,
    ToDef, Transform,
};

use crate::admin::dsl::models::{
    DslFieldSpec, DslOptionsAndExamplesResponse, DslOptionsResponse, DslTypeSpec,
    DslValidateRequest, DslValidateResponse,
};

/// Build field specifications for format FROM type
fn build_format_from_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "source.source_type".into(),
            r#type: "string".into(),
            required: true,
            options: Some(vec!["uri".into(), "api".into(), "file".into()]),
        },
        DslFieldSpec {
            name: "source.config.uri".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "source.config.endpoint".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "source.auth".into(),
            r#type: "object".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.format_type".into(),
            r#type: "string".into(),
            required: true,
            options: Some(vec!["csv".into(), "json".into()]),
        },
        DslFieldSpec {
            name: "format.options.has_header".into(),
            r#type: "boolean".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.options.delimiter".into(),
            r#type: "string(1)".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.options.escape".into(),
            r#type: "string(1)".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.options.quote".into(),
            r#type: "string(1)".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "mapping".into(),
            r#type: "map<string,string>".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build field specifications for entity FROM type
fn build_entity_from_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "entity_definition".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "filter".into(),
            r#type: "object".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "mapping".into(),
            r#type: "map<string,string>".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build FROM type specifications
fn build_from_type_specs() -> Vec<DslTypeSpec> {
    vec![
        DslTypeSpec {
            r#type: "format".to_string(),
            fields: build_format_from_fields(),
        },
        DslTypeSpec {
            r#type: "entity".to_string(),
            fields: build_entity_from_fields(),
        },
    ]
}

/// Build field specifications for format TO type
fn build_format_to_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "output.mode".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec!["api".into(), "download".into(), "push".into()]),
        },
        DslFieldSpec {
            name: "output.push.destination.destination_type".into(),
            r#type: "string".into(),
            required: false,
            options: Some(vec!["uri".into()]),
        },
        DslFieldSpec {
            name: "output.push.destination.config.uri".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "output.push.method".into(),
            r#type: "enum".into(),
            required: false,
            options: Some(vec![
                "GET".into(),
                "POST".into(),
                "PUT".into(),
                "PATCH".into(),
                "DELETE".into(),
                "HEAD".into(),
                "OPTIONS".into(),
            ]),
        },
        DslFieldSpec {
            name: "output.push.destination.auth".into(),
            r#type: "object".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.format_type".into(),
            r#type: "string".into(),
            required: true,
            options: Some(vec!["csv".into(), "json".into()]),
        },
        DslFieldSpec {
            name: "format.options.has_header".into(),
            r#type: "boolean".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.options.delimiter".into(),
            r#type: "string(1)".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.options.escape".into(),
            r#type: "string(1)".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "format.options.quote".into(),
            r#type: "string(1)".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "mapping".into(),
            r#type: "map<string,string>".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build field specifications for entity TO type
fn build_entity_to_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "entity_definition".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "path".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "mode".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec!["create".into(), "update".into()]),
        },
        DslFieldSpec {
            name: "identify.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "identify.value".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "update_key".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "mapping".into(),
            r#type: "map<string,string>".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build TO type specifications
fn build_to_type_specs() -> Vec<DslTypeSpec> {
    vec![
        DslTypeSpec {
            r#type: "format".to_string(),
            fields: build_format_to_fields(),
        },
        DslTypeSpec {
            r#type: "entity".to_string(),
            fields: build_entity_to_fields(),
        },
    ]
}

/// Build field specifications for arithmetic transform type
fn build_arithmetic_transform_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "target".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "left.kind".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec![
                "field".into(),
                "const".into(),
                "external_entity_field".into(),
            ]),
        },
        DslFieldSpec {
            name: "left.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "left.value".into(),
            r#type: "number".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "left.entity_definition".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "left.filter.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "left.filter.value".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "op".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec!["add".into(), "sub".into(), "mul".into(), "div".into()]),
        },
        DslFieldSpec {
            name: "right.kind".into(),
            r#type: "enum".into(),
            required: true,
            options: Some(vec![
                "field".into(),
                "const".into(),
                "external_entity_field".into(),
            ]),
        },
        DslFieldSpec {
            name: "right.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "right.value".into(),
            r#type: "number".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "right.entity_definition".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "right.filter.field".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "right.filter.value".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
    ]
}

/// Build field specifications for concat transform type
fn build_concat_transform_fields() -> Vec<DslFieldSpec> {
    vec![
        DslFieldSpec {
            name: "target".into(),
            r#type: "string".into(),
            required: true,
            options: None,
        },
        DslFieldSpec {
            name: "separator".into(),
            r#type: "string".into(),
            required: false,
            options: None,
        },
        DslFieldSpec {
            name: "fields".into(),
            r#type: "array<string>".into(),
            required: true,
            options: None,
        },
    ]
}

/// Build transform type specifications
fn build_transform_type_specs() -> Vec<DslTypeSpec> {
    vec![
        DslTypeSpec {
            r#type: "none".to_string(),
            fields: vec![],
        },
        DslTypeSpec {
            r#type: "arithmetic".to_string(),
            fields: build_arithmetic_transform_fields(),
        },
        DslTypeSpec {
            r#type: "concat".to_string(),
            fields: build_concat_transform_fields(),
        },
    ]
}

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

    // Convert Vec<Value> to Vec<DslStep> for validation
    let steps: Result<Vec<DslStep>, _> = payload
        .steps
        .iter()
        .map(|v| serde_json::from_value(v.clone()))
        .collect();
    let Ok(steps) = steps else {
        return ApiResponse::<()>::unprocessable_entity("Invalid DSL steps format");
    };
    let program = DslProgram { steps };
    match program.validate() {
        Ok(()) => ApiResponse::ok(DslValidateResponse { valid: true }),
        Err(e) => ApiResponse::<()>::unprocessable_entity_with_violations(
            "Invalid DSL",
            vec![ValidationViolation {
                field: "dsl".to_string(),
                message: e.to_string(),
                code: Some("DSL_INVALID".to_string()),
            }],
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
        filter: EntityFilter {
            field: "sku".to_string(),
            value: "ABC-001".to_string(),
        },
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
#[get("/to/options")]
pub async fn list_to_options(auth: RequiredAuth) -> impl Responder {
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
        types: build_to_type_specs(),
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
        path: "/".to_string(),
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
#[get("/transform/options")]
pub async fn list_transform_options(auth: RequiredAuth) -> impl Responder {
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
        types: build_transform_type_specs(),
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
    let resp = DslOptionsAndExamplesResponse {
        types: types.types,
        examples: vec![ex_arith, ex_concat],
    };
    ApiResponse::ok(resp)
}

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .service(validate_dsl)
            .service(list_from_options)
            .service(list_to_options)
            .service(list_transform_options),
    );
}
