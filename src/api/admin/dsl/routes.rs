use actix_web::{get, post, web, Responder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::api::auth::auth_enum;
use crate::api::response::{ApiResponse, ValidationViolation};
use crate::workflow::dsl::{
    ArithmeticOp, ArithmeticTransform, ConcatTransform, DslProgram, DslStep, EntityFilter,
    EntityWriteMode, FromDef, Operand, OutputMode, StringOperand, ToDef, Transform,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct DslValidateRequest {
    /// The DSL steps array (JSON). Example: { "steps": [ { "from": { ... }, "transform": { ... }, "to": { ... } } ] }
    #[schema(value_type = Vec<DslStep>, example = json!([
        {
            "from": { "type": "csv", "uri": "http://example.com/data.csv", "mapping": { "price": "price" } },
            "transform": {
                "type": "arithmetic",
                "target": "price",
                "left": { "kind": "field", "field": "price" },
                "op": "add",
                "right": { "kind": "const", "value": 5.0 }
            },
            "to": { "type": "json", "output": "api", "mapping": { "price": "entity.total" } }
        }
    ]))]
    pub steps: Vec<DslStep>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslValidateResponse {
    /// Whether the DSL is valid
    pub valid: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslFieldSpec {
    pub name: String,
    #[schema(example = "string")]
    pub r#type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslTypeSpec {
    pub r#type: String,
    pub fields: Vec<DslFieldSpec>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslOptionsResponse {
    pub types: Vec<DslTypeSpec>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DslOptionsAndExamplesResponse {
    pub types: Vec<DslTypeSpec>,
    /// Concrete serialized examples using the real DSL structs
    pub examples: Vec<Value>,
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
    _: auth_enum::RequiredAuth,
) -> impl Responder {
    let program = DslProgram {
        steps: payload.steps.clone(),
    };
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
        (status = 200, description = "Available FROM types and field specs", body = DslOptionsResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("jwt" = []))
)]
#[get("/from/options")]
pub async fn list_from_options(_: auth_enum::RequiredAuth) -> impl Responder {
    let types = DslOptionsResponse {
        types: vec![
            DslTypeSpec {
                r#type: "csv".to_string(),
                fields: vec![
                    DslFieldSpec {
                        name: "uri".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "mapping".to_string(),
                        r#type: "map<string,string>".to_string(),
                        required: true,
                        options: None,
                    },
                ],
            },
            DslTypeSpec {
                r#type: "json".to_string(),
                fields: vec![
                    DslFieldSpec {
                        name: "uri".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "mapping".to_string(),
                        r#type: "map<string,string>".to_string(),
                        required: true,
                        options: None,
                    },
                ],
            },
            DslTypeSpec {
                r#type: "entity".to_string(),
                fields: vec![
                    DslFieldSpec {
                        name: "entity_definition".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "filter.field".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "filter.value".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "mapping".to_string(),
                        r#type: "map<string,string>".to_string(),
                        required: true,
                        options: None,
                    },
                ],
            },
        ],
    };
    let ex_csv = serde_json::to_value(FromDef::Csv {
        uri: "http://example.com/data.csv".to_string(),
        mapping: [("price".to_string(), "price".to_string())]
            .into_iter()
            .collect(),
    })
    .unwrap();
    let ex_json = serde_json::to_value(FromDef::Json {
        uri: "http://example.com/data.json".to_string(),
        mapping: [("amount".to_string(), "amount".to_string())]
            .into_iter()
            .collect(),
    })
    .unwrap();
    let ex_entity = serde_json::to_value(FromDef::Entity {
        entity_definition: "product".to_string(),
        filter: EntityFilter {
            field: "sku".to_string(),
            value: "ABC-001".to_string(),
        },
        mapping: [("price".to_string(), "price".to_string())]
            .into_iter()
            .collect(),
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
        (status = 200, description = "Available TO types and field specs", body = DslOptionsResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("jwt" = []))
)]
#[get("/to/options")]
pub async fn list_to_options(_: auth_enum::RequiredAuth) -> impl Responder {
    let types = DslOptionsResponse {
        types: vec![
            DslTypeSpec {
                r#type: "csv".to_string(),
                fields: vec![
                    DslFieldSpec {
                        name: "output".to_string(),
                        r#type: "enum".to_string(),
                        required: true,
                        options: Some(vec!["api".into(), "download".into()]),
                    },
                    DslFieldSpec {
                        name: "mapping".to_string(),
                        r#type: "map<string,string>".to_string(),
                        required: true,
                        options: None,
                    },
                ],
            },
            DslTypeSpec {
                r#type: "json".to_string(),
                fields: vec![
                    DslFieldSpec {
                        name: "output".to_string(),
                        r#type: "enum".to_string(),
                        required: true,
                        options: Some(vec!["api".into(), "download".into()]),
                    },
                    DslFieldSpec {
                        name: "mapping".to_string(),
                        r#type: "map<string,string>".to_string(),
                        required: true,
                        options: None,
                    },
                ],
            },
            DslTypeSpec {
                r#type: "entity".to_string(),
                fields: vec![
                    DslFieldSpec {
                        name: "entity_definition".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "path".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "mode".to_string(),
                        r#type: "enum".to_string(),
                        required: true,
                        options: Some(vec!["create".into(), "update".into()]),
                    },
                    DslFieldSpec {
                        name: "identify.field".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "identify.value".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "mapping".to_string(),
                        r#type: "map<string,string>".to_string(),
                        required: true,
                        options: None,
                    },
                ],
            },
        ],
    };
    let ex_csv = serde_json::to_value(ToDef::Csv {
        output: OutputMode::Api,
        mapping: [("price".to_string(), "entity.total".to_string())]
            .into_iter()
            .collect(),
    })
    .unwrap();
    let ex_json = serde_json::to_value(ToDef::Json {
        output: OutputMode::Api,
        mapping: [("price".to_string(), "entity.total".to_string())]
            .into_iter()
            .collect(),
    })
    .unwrap();
    let ex_entity = serde_json::to_value(ToDef::Entity {
        entity_definition: "product".to_string(),
        path: "/".to_string(),
        mode: EntityWriteMode::Create,
        identify: None,
        mapping: [("price".to_string(), "price".to_string())]
            .into_iter()
            .collect(),
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
        (status = 200, description = "Available TRANSFORM types and field specs", body = DslOptionsResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("jwt" = []))
)]
#[get("/transform/options")]
pub async fn list_transform_options(_: auth_enum::RequiredAuth) -> impl Responder {
    let types = DslOptionsResponse {
        types: vec![
            DslTypeSpec {
                r#type: "none".to_string(),
                fields: vec![],
            },
            DslTypeSpec {
                r#type: "arithmetic".to_string(),
                fields: vec![
                    DslFieldSpec {
                        name: "target".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "left.kind".to_string(),
                        r#type: "enum".to_string(),
                        required: true,
                        options: Some(vec![
                            "field".into(),
                            "const".into(),
                            "external_entity_field".into(),
                        ]),
                    },
                    DslFieldSpec {
                        name: "left.field".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "left.value".to_string(),
                        r#type: "number".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "left.entity_definition".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "left.filter.field".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "left.filter.value".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "op".to_string(),
                        r#type: "enum".to_string(),
                        required: true,
                        options: Some(vec!["add".into(), "sub".into(), "mul".into(), "div".into()]),
                    },
                    DslFieldSpec {
                        name: "right.kind".to_string(),
                        r#type: "enum".to_string(),
                        required: true,
                        options: Some(vec![
                            "field".into(),
                            "const".into(),
                            "external_entity_field".into(),
                        ]),
                    },
                    DslFieldSpec {
                        name: "right.field".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "right.value".to_string(),
                        r#type: "number".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "right.entity_definition".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "right.filter.field".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "right.filter.value".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                ],
            },
            DslTypeSpec {
                r#type: "concat".to_string(),
                fields: vec![
                    DslFieldSpec {
                        name: "target".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "left.kind".to_string(),
                        r#type: "enum".to_string(),
                        required: true,
                        options: Some(vec!["field".into(), "const_string".into()]),
                    },
                    DslFieldSpec {
                        name: "left.field".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "left.value".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "separator".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "right.kind".to_string(),
                        r#type: "enum".to_string(),
                        required: true,
                        options: Some(vec!["field".into(), "const_string".into()]),
                    },
                    DslFieldSpec {
                        name: "right.field".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                    DslFieldSpec {
                        name: "right.value".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        options: None,
                    },
                ],
            },
        ],
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
