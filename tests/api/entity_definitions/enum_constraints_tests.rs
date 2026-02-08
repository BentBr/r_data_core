#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Tests for enum/select field constraints.

use super::common::{create_test_app, create_test_jwt_token};
use actix_web::{
    http::{header, StatusCode},
    test,
};
use r_data_core_core::error::Result;
use r_data_core_test_support::{clear_test_db, create_test_admin_user, setup_test_db};
use serial_test::serial;

/// Test creating entity definition with select/enum constraints.
#[tokio::test]
#[serial]
async fn test_create_entity_definition_with_enum_constraints() -> Result<()> {
    let pool = setup_test_db().await;
    let user_uuid = create_test_admin_user(&pool).await?;
    let app = create_test_app(&pool).await;
    let token = create_test_jwt_token(&user_uuid, "test_secret");

    let payload = serde_json::json!({
        "entity_type": "test_order",
        "display_name": "Test Order",
        "description": "Order with status enum",
        "group_name": "test",
        "allow_children": false,
        "icon": "shopping-cart",
        "fields": [
            {
                "name": "status",
                "display_name": "Status",
                "field_type": "String",
                "required": true,
                "indexed": true,
                "filterable": true,
                "constraints": {
                    "type": "string",
                    "constraints": {
                        "options": ["pending", "processing", "shipped", "delivered", "cancelled"]
                    }
                }
            }
        ],
        "published": true
    });

    let req = test::TestRequest::post()
        .uri("/admin/api/v1/entity-definitions")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "Success");

    clear_test_db(&pool).await?;
    Ok(())
}
